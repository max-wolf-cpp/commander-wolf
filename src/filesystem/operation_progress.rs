use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::file_operations::progress::{OperationObserver, OperationProgress};
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationKind, FileOperationOutcome,
    FileOperationRequest, OperationStatus,
};

struct Reporter<'a> {
    cancel: &'a AtomicBool,
    notify: &'a mut dyn FnMut(OperationProgress),
    progress: OperationProgress,
    root_limit: u64,
    operation_start: u64,
    count_work: bool,
}

#[derive(Default)]
struct SequenceResults {
    succeeded: usize,
    changed: bool,
    errors: Vec<String>,
    last_outcome: Option<FileOperationOutcome>,
}

pub fn execute_batch_with_progress(
    request: &FileOperationBatchRequest,
    cancel: &AtomicBool,
    notify: &mut dyn FnMut(OperationProgress),
) -> FileOperationOutcome {
    let Some(kind) = request.kind() else {
        return FileOperationOutcome::failure("File operation batch is empty");
    };

    if request
        .operations
        .iter()
        .any(|operation| operation.kind() != kind)
    {
        return FileOperationOutcome::failure(
            "File operation batch contains mixed operation kinds",
        );
    }

    execute_sequence_with_progress(request, cancel, notify, false)
}

pub(super) fn execute_sequence_with_progress(
    request: &FileOperationBatchRequest,
    cancel: &AtomicBool,
    notify: &mut dyn FnMut(OperationProgress),
    stop_on_failure: bool,
) -> FileOperationOutcome {
    let Some(kind) = request.kind() else {
        return FileOperationOutcome::failure("File operation sequence is empty");
    };

    let mut reporter = Reporter {
        cancel,
        notify,
        progress: OperationProgress::new(kind),
        root_limit: u64::MAX,
        operation_start: 0,
        count_work: true,
    };
    let weights = match reporter.scan_operations(&request.operations) {
        Ok(weights) => weights,
        Err(error) => return FileOperationOutcome::failure(error),
    };

    let mut results = SequenceResults::default();
    for (operation, weight) in request.operations.iter().zip(weights) {
        if reporter.check_cancelled().is_err() {
            break;
        }

        reporter.begin_operation(operation, weight);

        let outcome = super::operations::execute_operation_observed(
            &FileOperationRequest {
                operation: operation.clone(),
                protected_directory: request.protected_directory.clone(),
                refresh: request.refresh.clone(),
            },
            &mut reporter,
        );

        results.record(outcome);
        if stop_on_failure && !results.errors.is_empty() {
            break;
        }

        if reporter.check_cancelled().is_err() {
            break;
        }

        reporter.progress.total_done = reporter.root_limit.min(reporter.progress.total);
        reporter.emit();
    }

    results.outcome(
        kind,
        request.operations.len(),
        cancel.load(Ordering::Relaxed),
    )
}

impl SequenceResults {
    fn record(&mut self, outcome: FileOperationOutcome) {
        self.changed |= outcome.filesystem_changed;
        if outcome.status == OperationStatus::Success {
            self.succeeded += 1;
        } else {
            self.errors.push(outcome.message.clone());
        }
        self.last_outcome = Some(outcome);
    }

    fn outcome(
        self,
        kind: FileOperationKind,
        total: usize,
        cancelled: bool,
    ) -> FileOperationOutcome {
        let succeeded = self.succeeded;
        if cancelled {
            return FileOperationOutcome {
                status: if self.changed {
                    OperationStatus::Partial
                } else {
                    OperationStatus::Failure
                },
                filesystem_changed: self.changed,
                message: format!(
                    "Cancelled: {succeeded}/{} operations completed. Completed changes were kept. {}",
                    total,
                    self.errors.join("; ")
                ),
            };
        }

        if total == 1 {
            return self
                .last_outcome
                .unwrap_or_else(|| FileOperationOutcome::failure("No operation was executed"));
        }

        if self.errors.is_empty() {
            return FileOperationOutcome::success(format!(
                "Applied {succeeded} {kind:?} operation(s)"
            ));
        }

        FileOperationOutcome {
            status: if self.changed {
                OperationStatus::Partial
            } else {
                OperationStatus::Failure
            },
            filesystem_changed: self.changed,
            message: format!(
                "{kind:?} batch: {succeeded} succeeded, {} failed: {}",
                self.errors.len(),
                self.errors.join("; ")
            ),
        }
    }
}

fn scan(path: &Path, kind: FileOperationKind, reporter: &mut Reporter<'_>) -> Result<u64, String> {
    reporter.check_cancelled()?;
    reporter.progress.path = path.to_path_buf();
    reporter.emit();
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_dir() && !super::is_link(&metadata) {
        let mut weight = 1u64;
        for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            weight = weight.saturating_add(scan(&entry.path(), kind, reporter)?);
        }
        Ok(weight)
    } else if metadata.is_file() && kind != FileOperationKind::Delete {
        Ok(metadata.len().max(1))
    } else {
        Ok(1)
    }
}

impl Reporter<'_> {
    fn scan_operations(&mut self, operations: &[FileOperation]) -> Result<Vec<u64>, String> {
        self.progress.phase = "Scanning".into();
        let mut weights = Vec::with_capacity(operations.len());
        for operation in operations {
            let weight = operation
                .source()
                .map_or(Ok(1), |path| scan(path, operation.kind(), self));
            if self.cancel.load(Ordering::Relaxed) {
                return Err("Cancelled before any filesystem changes".into());
            }

            // Unreadable entries still go through validation and produce their normal error.
            weights.push(weight.unwrap_or(1));
        }
        self.progress.total = weights
            .iter()
            .fold(0u64, |sum, weight| sum.saturating_add(*weight))
            .max(1);

        Ok(weights)
    }

    fn begin_operation(&mut self, operation: &FileOperation, weight: u64) {
        self.operation_start = self.progress.total_done;
        self.root_limit = self.progress.total_done.saturating_add(weight);
        self.count_work = true;
        self.progress.kind = operation.kind();
        self.phase(match operation.kind() {
            FileOperationKind::Delete => "Deleting",
            FileOperationKind::Move => "Moving",
            FileOperationKind::Copy => "Copying",
            FileOperationKind::Rename => "Renaming",
            FileOperationKind::CreateArchive => "Archiving",
            FileOperationKind::ExtractArchive => "Extracting",
            _ => "Creating",
        });
        if let Some(path) = operation.source() {
            self.begin_file(path, 1);
        }
    }

    fn emit(&mut self) {
        (self.notify)(self.progress.clone());
    }
}

impl OperationObserver for Reporter<'_> {
    fn set_total(&mut self, size: u64) {
        let previous = self.root_limit.saturating_sub(self.operation_start);
        let size = size.max(1);
        self.progress.total = self
            .progress
            .total
            .saturating_sub(previous)
            .saturating_add(size);
        self.root_limit = self.operation_start.saturating_add(size);
        self.emit();
    }

    fn check_cancelled(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Operation cancelled".into())
        } else {
            Ok(())
        }
    }

    fn begin_file(&mut self, path: &Path, size: u64) {
        self.progress.path = path.to_path_buf();
        self.progress.file_done = 0;
        self.progress.file_total = size.max(1);
        self.emit();
    }

    fn advance(&mut self, bytes: u64) {
        let delta = bytes.min(
            self.progress
                .file_total
                .saturating_sub(self.progress.file_done),
        );
        self.progress.file_done += delta;
        if self.count_work {
            self.progress.total_done = self
                .progress
                .total_done
                .saturating_add(delta)
                .min(self.root_limit)
                .min(self.progress.total);
        }
        self.emit();
    }

    fn finish_file(&mut self) {
        self.advance(
            self.progress
                .file_total
                .saturating_sub(self.progress.file_done),
        );
    }

    fn phase(&mut self, phase: &str) {
        self.progress.phase = phase.into();
        self.count_work = phase != "Removing source";
        self.emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_operations::{FileOperation, PanelRefreshRequest, RefreshSelection};
    use crate::workspace::PanelId;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "commander-wolf-progress-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn request(root: &Path, operations: Vec<FileOperation>) -> FileOperationBatchRequest {
        FileOperationBatchRequest {
            operations,
            source_panel: PanelId::Left,
            protected_directory: root.into(),
            refresh: [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
                panel,
                directory: root.into(),
                selection: RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            }),
        }
    }

    #[test]
    fn totals_cover_nested_files_empty_files_and_multiple_roots() {
        let root = TestDirectory::new();
        let tree = root.0.join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(tree.join("empty"), []).unwrap();
        fs::write(tree.join("data"), vec![42; 512 * 1024]).unwrap();
        let other = root.0.join("other");
        fs::write(&other, "12345").unwrap();
        let request = request(
            &root.0,
            vec![
                FileOperation::Copy {
                    source: tree,
                    destination: root.0.join("copied-tree"),
                    overwrite: false,
                },
                FileOperation::Copy {
                    source: other,
                    destination: root.0.join("copied-other"),
                    overwrite: false,
                },
            ],
        );
        let mut updates = Vec::new();
        let outcome =
            execute_batch_with_progress(&request, &AtomicBool::new(false), &mut |progress| {
                updates.push(progress)
            });
        assert_eq!(outcome.status, OperationStatus::Success);
        let last = updates.last().unwrap();
        assert_eq!(last.total, 512 * 1024 + 7);
        assert_eq!(last.total_done, last.total);
        assert!(updates.iter().any(|progress| progress.file_done > 0));
        // Windows chooses native callback granularity, potentially copying a small file at once.
        #[cfg(not(windows))]
        assert!(
            updates
                .iter()
                .any(|progress| progress.file_done > 0 && progress.file_done < progress.file_total)
        );
        assert!(
            updates
                .windows(2)
                .all(|pair| pair[0].total_done <= pair[1].total_done)
        );
        assert!(
            updates
                .iter()
                .all(|progress| progress.total_done <= progress.total)
        );
    }

    #[test]
    fn cancelling_during_scan_changes_nothing_and_cancelling_a_batch_stops_remaining_roots() {
        let root = TestDirectory::new();
        let first = root.0.join("first");
        let second = root.0.join("second");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        let request = request(
            &root.0,
            vec![
                FileOperation::Delete {
                    source: first.clone(),
                },
                FileOperation::Delete {
                    source: second.clone(),
                },
            ],
        );
        let cancel = AtomicBool::new(false);
        let outcome = execute_batch_with_progress(&request, &cancel, &mut |_| {
            cancel.store(true, Ordering::Relaxed);
        });
        assert!(!outcome.filesystem_changed);
        assert!(first.exists() && second.exists());
        cancel.store(false, Ordering::Relaxed);
        let outcome = execute_batch_with_progress(&request, &cancel, &mut |progress| {
            if progress.total_done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert_eq!(outcome.status, OperationStatus::Partial);
        assert!(!first.exists() && second.exists());
        assert!(outcome.message.contains("Cancelled"));
    }

    #[test]
    fn same_device_moves_and_deletes_reach_complete_totals() {
        let root = TestDirectory::new();
        let source = root.0.join("source");
        let destination = root.0.join("destination");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("data"), "123").unwrap();
        for operation in [
            FileOperation::Move {
                source: source.clone(),
                destination: destination.clone(),
                overwrite: false,
            },
            FileOperation::Delete {
                source: destination.clone(),
            },
        ] {
            let mut last = None;
            let outcome = execute_batch_with_progress(
                &request(&root.0, vec![operation]),
                &AtomicBool::new(false),
                &mut |progress| last = Some(progress),
            );
            assert_eq!(outcome.status, OperationStatus::Success);
            let last = last.unwrap();
            assert_eq!(last.total_done, last.total);
        }
        assert!(!source.exists() && !destination.exists());
    }
}
