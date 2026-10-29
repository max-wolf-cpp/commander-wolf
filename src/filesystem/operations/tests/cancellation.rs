use super::*;

struct CancelObserver {
    cancelled: bool,
    bytes: u64,
    deleted: usize,
    stop_after_copy_chunk: bool,
    stop_before_source_removal: bool,
    stop_after_delete: bool,
}

impl OperationObserver for CancelObserver {
    fn check_cancelled(&self) -> Result<(), String> {
        if self.cancelled {
            Err("Operation cancelled".into())
        } else {
            Ok(())
        }
    }

    fn advance(&mut self, bytes: u64) {
        self.bytes += bytes;
        if self.stop_after_copy_chunk {
            self.cancelled = true;
        }
    }

    fn finish_file(&mut self) {
        self.deleted += 1;
        if self.stop_after_delete {
            self.cancelled = true;
        }
    }

    fn phase(&mut self, phase: &str) {
        if self.stop_before_source_removal && phase == "Removing source" {
            self.cancelled = true;
        }
    }
}

fn cancel_observer() -> CancelObserver {
    CancelObserver {
        cancelled: false,
        bytes: 0,
        deleted: 0,
        stop_after_copy_chunk: false,
        stop_before_source_removal: false,
        stop_after_delete: false,
    }
}

#[test]
fn cancelling_copy_removes_staging_and_preserves_both_originals() {
    let root = TestDirectory::new();
    let source = root.0.join("source.bin");
    let destination = root.0.join("destination.bin");
    let bytes = vec![42; 1024 * 1024];
    fs::write(&source, &bytes).unwrap();
    fs::write(&destination, "previous contents").unwrap();
    let mut observer = cancel_observer();
    observer.stop_after_copy_chunk = true;

    let outcome = execute_operation_observed(
        &request(
            FileOperation::Copy {
                source: source.clone(),
                destination: destination.clone(),
                overwrite: true,
            },
            &root.0,
        ),
        &mut observer,
    );

    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(!outcome.filesystem_changed);
    assert!(outcome.message.contains("cancelled"));
    assert!(observer.bytes > 0 && observer.bytes <= bytes.len() as u64);
    #[cfg(not(windows))]
    assert!(observer.bytes < bytes.len() as u64);
    assert_eq!(fs::read(&source).unwrap(), bytes);
    assert_eq!(
        fs::read_to_string(destination).unwrap(),
        "previous contents"
    );

    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2);
}

#[test]
fn cancelling_delete_stops_between_entries_and_reports_partial_changes() {
    let root = TestDirectory::new();
    let source = root.0.join("tree");
    fs::create_dir(&source).unwrap();
    for name in ["one", "two", "three"] {
        fs::write(source.join(name), "data").unwrap();
    }
    let mut observer = cancel_observer();
    observer.stop_after_delete = true;

    let outcome = execute_operation_observed(
        &request(
            FileOperation::Delete {
                source: source.clone(),
            },
            &root.0,
        ),
        &mut observer,
    );

    assert_eq!(outcome.status, OperationStatus::Partial);
    assert!(outcome.filesystem_changed);
    assert_eq!(observer.deleted, 1);
    assert_eq!(fs::read_dir(source).unwrap().count(), 2);
}

#[test]
fn cancelled_cross_device_move_keeps_published_copy_and_remaining_source() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let destination = root.0.join("destination");
    fs::write(&source, "complete data").unwrap();
    let mut observer = cancel_observer();
    observer.stop_before_source_removal = true;

    let outcome = move_path_with(
        &source,
        &destination,
        |_, _| Err(io::ErrorKind::CrossesDevices.into()),
        remove_source_observed,
        &mut observer,
    );

    assert_eq!(outcome.status, OperationStatus::Partial);
    assert_eq!(fs::read_to_string(source).unwrap(), "complete data");
    assert_eq!(fs::read_to_string(destination).unwrap(), "complete data");
}

#[test]
fn finalization_defers_cancellation_until_the_published_copy_is_complete() {
    use std::sync::atomic::AtomicBool;

    let root = TestDirectory::new();
    let source = root.0.join("source");
    let dest = root.0.join("dest");
    fs::write(&source, "new").unwrap();
    fs::write(&dest, "old").unwrap();
    let single = request(
        FileOperation::Copy {
            source: source.clone(),
            destination: dest.clone(),
            overwrite: true,
        },
        &root.0,
    );
    let request = FileOperationBatchRequest {
        operations: vec![single.operation],
        source_panel: PanelId::Left,
        protected_directory: single.protected_directory,
        refresh: single.refresh,
    };
    let cancelled = AtomicBool::new(false);

    let outcome =
        crate::filesystem::execute_batch_with_progress(&request, &cancelled, &mut |progress| {
            if progress.phase == "Finalizing" {
                cancelled.store(true, Ordering::Relaxed);
            }
        });
    assert_eq!(outcome.status, OperationStatus::Partial);
    assert!(outcome.filesystem_changed);
    assert!(outcome.message.contains("1/1"));
    assert_eq!(fs::read_to_string(source).unwrap(), "new");
    assert_eq!(fs::read_to_string(dest).unwrap(), "new");
    assert!(artifacts(&root.0).is_empty());
}
