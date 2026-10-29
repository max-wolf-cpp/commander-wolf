use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

use crate::directory_edit::DirectoryEditRequest;
use crate::file_operations::progress::OperationProgress;
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationOutcome, FileOperationRequest,
    OperationStatus, PanelRefreshResult,
};

pub fn execute(request: &DirectoryEditRequest) -> (FileOperationOutcome, [PanelRefreshResult; 2]) {
    let operations = match ordered_operations(&request.operations) {
        Ok(operations) => operations,
        Err(message) => {
            return (
                FileOperationOutcome::failure(message),
                super::directory::refresh_panels(request.refresh.clone()),
            );
        }
    };

    let mut changed = 0usize;
    let mut messages = Vec::new();
    let mut failure = None;
    for operation in &operations {
        let outcome = super::operations::execute_operation(&FileOperationRequest {
            operation: operation.clone(),
            protected_directory: request.protected_directory.clone(),
            refresh: request.refresh.clone(),
        });
        messages.push(outcome.message);
        if outcome.filesystem_changed {
            changed += 1;
        }
        if outcome.status != OperationStatus::Success {
            failure = Some(outcome.status);
            break;
        }
    }

    let outcome = edit_outcome(changed, messages, failure);

    (
        outcome,
        super::directory::refresh_panels(request.refresh.clone()),
    )
}

pub fn execute_with_progress(
    request: &DirectoryEditRequest,
    cancel: &AtomicBool,
    notify: &mut dyn FnMut(OperationProgress),
) -> FileOperationOutcome {
    let operations = match ordered_operations(&request.operations) {
        Ok(operations) => operations,
        Err(error) => return FileOperationOutcome::failure(error),
    };

    let count = operations.len();
    let mut outcome = super::operation_progress::execute_sequence_with_progress(
        &FileOperationBatchRequest {
            operations,
            source_panel: request.refresh[0].panel,
            protected_directory: request.protected_directory.clone(),
            refresh: request.refresh.clone(),
        },
        cancel,
        notify,
        true,
    );

    if outcome.status == OperationStatus::Success {
        outcome.message = format!("Applied {count} directory edit operation(s)");
    }

    outcome
}

fn edit_outcome(
    changed: usize,
    messages: Vec<String>,
    failure: Option<OperationStatus>,
) -> FileOperationOutcome {
    if let Some(status) = failure {
        FileOperationOutcome {
            status: if changed > 0 {
                OperationStatus::Partial
            } else {
                status
            },
            filesystem_changed: changed > 0,
            message: messages.join("; "),
        }
    } else {
        FileOperationOutcome {
            status: OperationStatus::Success,
            filesystem_changed: changed > 0,
            message: if changed == 0 {
                "Directory edit contained no changes".into()
            } else {
                format!("Applied {changed} directory edit operation(s)")
            },
        }
    }
}

fn ordered_operations(operations: &[FileOperation]) -> Result<Vec<FileOperation>, String> {
    validate_destinations(operations)?;

    let mut ordered = operations
        .iter()
        .filter(|operation| matches!(operation, FileOperation::Delete { .. }))
        .cloned()
        .collect::<Vec<_>>();
    order_renames(operations, &mut ordered)?;

    ordered.extend(
        operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation,
                    FileOperation::CreateDirectory { .. } | FileOperation::CreateFile { .. }
                )
            })
            .cloned(),
    );

    Ok(ordered)
}

fn validate_destinations(operations: &[FileOperation]) -> Result<(), String> {
    let sources = operations
        .iter()
        .filter_map(FileOperation::source)
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();

    for operation in operations {
        let destination = match operation {
            FileOperation::Rename { destination, .. }
            | FileOperation::CreateDirectory { destination }
            | FileOperation::CreateFile { destination }
            | FileOperation::CreateArchive { destination, .. }
            | FileOperation::ExtractArchive { destination, .. } => Some(destination),
            FileOperation::Copy { destination, .. } | FileOperation::Move { destination, .. } => {
                Some(destination)
            }
            FileOperation::Delete { .. } => None,
        };
        if let Some(destination) = destination
            && destination.exists()
            && !sources.contains(destination)
        {
            return Err(format!(
                "Directory edit destination already exists: '{}'",
                destination.display()
            ));
        }
    }

    Ok(())
}

fn order_renames(
    operations: &[FileOperation],
    ordered: &mut Vec<FileOperation>,
) -> Result<(), String> {
    let mut renames = operations
        .iter()
        .filter(|operation| matches!(operation, FileOperation::Rename { .. }))
        .cloned()
        .collect::<Vec<_>>();
    while !renames.is_empty() {
        let position = renames
            .iter()
            .position(|candidate| {
                let FileOperation::Rename { destination, .. } = candidate else {
                    return false;
                };

                !renames.iter().any(|other| {
                    matches!(
                        other,
                        FileOperation::Rename { source, .. } if source == destination
                    )
                })
            })
            .ok_or_else(|| {
                "Cyclic directory renames cannot be applied safely in one edit".to_owned()
            })?;

        ordered.push(renames.remove(position));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::file_operations::{FileOperation, PanelRefreshRequest, RefreshSelection};
    use crate::workspace::PanelId;

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("commander-wolf-edit-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();

            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn cancelling_observed_edit_keeps_completed_delete_and_stops_later_changes() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let root = Temp::new();
        let deleted = root.0.join("deleted");
        let original = root.0.join("original");
        let renamed = root.0.join("renamed");
        fs::write(&deleted, "delete").unwrap();
        fs::write(&original, "keep").unwrap();

        let request = DirectoryEditRequest {
            operations: vec![
                FileOperation::Rename {
                    source: original.clone(),
                    destination: renamed.clone(),
                },
                FileOperation::Delete {
                    source: deleted.clone(),
                },
                FileOperation::CreateFile {
                    destination: root.0.join("created"),
                },
            ],
            protected_directory: root.0.clone(),
            refresh: [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
                panel,
                directory: root.0.clone(),
                selection: RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            }),
        };

        let cancel = AtomicBool::new(false);
        let outcome = execute_with_progress(&request, &cancel, &mut |progress| {
            if progress.total_done > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });

        assert_eq!(outcome.status, OperationStatus::Partial);
        assert!(outcome.filesystem_changed);
        assert!(outcome.message.contains("Cancelled"));
        assert!(!deleted.exists());
        assert!(original.exists());
        assert!(!renamed.exists());
        assert!(!root.0.join("created").exists());
    }

    #[test]
    fn batch_applies_directory_text_diff_and_refreshes_once() {
        let root = Temp::new();
        let other = root.0.join("other");
        fs::create_dir(&other).unwrap();
        fs::write(root.0.join("old.txt"), "old").unwrap();
        fs::write(root.0.join("remove.txt"), "remove").unwrap();

        let refresh = [
            PanelRefreshRequest {
                panel: PanelId::Left,
                directory: root.0.clone(),
                selection: RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            },
            PanelRefreshRequest {
                panel: PanelId::Right,
                directory: other.clone(),
                selection: RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            },
        ];
        let request = DirectoryEditRequest {
            operations: vec![
                FileOperation::Delete {
                    source: root.0.join("remove.txt"),
                },
                FileOperation::Rename {
                    source: root.0.join("old.txt"),
                    destination: root.0.join("renamed.txt"),
                },
                FileOperation::CreateDirectory {
                    destination: root.0.join("new-dir"),
                },
                FileOperation::CreateFile {
                    destination: root.0.join("new.txt"),
                },
            ],
            protected_directory: other,
            refresh,
        };

        let (outcome, refreshed) = execute(&request);

        assert_eq!(outcome.status, OperationStatus::Success);
        assert!(root.0.join("renamed.txt").is_file());
        assert!(!root.0.join("remove.txt").exists());
        assert!(root.0.join("new-dir").is_dir());
        assert!(root.0.join("new.txt").is_file());
        assert!(refreshed.iter().all(|result| result.result.is_ok()));
    }

    #[test]
    fn partial_oil_save_discards_stale_batch_and_retry_uses_refreshed_files() {
        use crate::app::{Action, AppMode, AppState, Effect, update};
        use crate::terminal::TerminalSize;
        use crate::workspace::WorkspaceOptions;

        for asynchronous in [false, true] {
            let root = Temp::new();
            let deleted = root.0.join("deleted");
            let original = root.0.join("original");
            fs::write(&deleted, "old deletion").unwrap();
            fs::write(&original, "keep me").unwrap();

            let mut state = AppState::new(
                root.0.clone(),
                "test".into(),
                TerminalSize {
                    columns: 80,
                    rows: 24,
                },
                WorkspaceOptions::default(),
            );
            update(
                &mut state,
                crate::filesystem::load_directory(PanelId::Left, &root.0, None).unwrap(),
            );
            update(&mut state, Action::EditBegin);
            for line in &mut state.directory_edit.lines {
                if line.text == "deleted" {
                    line.text.clear();
                } else if line.text == "original" {
                    line.text = "renamed".into();
                }
            }
            update(&mut state, Action::EditRequestExit);
            let mut effects = update(&mut state, Action::AcceptConfirmation);
            let Effect::ApplyDirectoryEdit(request) = effects.remove(0) else {
                panic!("expected save")
            };

            let (outcome, results) = super::super::faults::with(
                |phase, _| {
                    if phase == "rename" {
                        Err(std::io::Error::other("injected rename failure"))
                    } else {
                        Ok(())
                    }
                },
                || {
                    if asynchronous {
                        let outcome = execute_with_progress(
                            &request,
                            &std::sync::atomic::AtomicBool::new(false),
                            &mut |_| {},
                        );
                        (
                            outcome,
                            crate::filesystem::refresh_panels(request.refresh.clone()),
                        )
                    } else {
                        execute(&request)
                    }
                },
            );

            assert_eq!(outcome.status, OperationStatus::Partial);
            assert!(outcome.filesystem_changed);
            assert!(!deleted.exists());
            assert_eq!(fs::read_to_string(&original).unwrap(), "keep me");

            update(
                &mut state,
                Action::DirectoryEditCompleted {
                    request,
                    outcome,
                    results: Box::new(results),
                },
            );
            assert_eq!(state.mode, AppMode::Normal);
            assert!(!state.directory_edit.active);
            assert!(
                state
                    .notification
                    .as_ref()
                    .unwrap()
                    .message
                    .contains("injected rename failure")
            );

            fs::write(&deleted, "new unrelated file").unwrap();
            update(&mut state, Action::EditBegin);
            for line in &mut state.directory_edit.lines {
                if line.text == "original" {
                    line.text = "renamed".into();
                }
            }
            update(&mut state, Action::EditRequestExit);
            let mut effects = update(&mut state, Action::AcceptConfirmation);
            let Effect::ApplyDirectoryEdit(request) = effects.remove(0) else {
                panic!("expected retry")
            };
            assert!(matches!(
                request.operations.as_slice(),
                [FileOperation::Rename { .. }]
            ));

            let (outcome, _) = execute(&request);

            assert_eq!(outcome.status, OperationStatus::Success);
            assert_eq!(fs::read_to_string(&deleted).unwrap(), "new unrelated file");
            assert_eq!(
                fs::read_to_string(root.0.join("renamed")).unwrap(),
                "keep me"
            );
        }
    }
}
