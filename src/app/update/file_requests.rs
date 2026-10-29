use super::file_operations::{begin_batch_operation, begin_operation};
use super::refresh::refresh_requests;
use crate::app::{Action, AppState, Effect, Notification};
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationKind, FileOperationRequest,
    RefreshSelection, normalize_from, validate_new_name,
};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::CopyToOther => batch_operation_from_targets(state, FileOperationKind::Copy),
        Action::MoveToOther => batch_operation_from_targets(state, FileOperationKind::Move),
        Action::Rename(new_name) => {
            if let Err(error) = validate_new_name(&new_name) {
                state.notification = Some(Notification::error(error));
                Vec::new()
            } else {
                operation_from_selection(state, FileOperationKind::Rename, Some(new_name))
            }
        }
        Action::CreateDirectory(path) => create_entry(state, path, true),
        Action::CreateFile(path) => create_entry(state, path, false),
        _ => unreachable!("dispatcher routed an unsupported action to file_requests"),
    }
}

fn operation_from_selection(
    state: &mut AppState,
    kind: FileOperationKind,
    argument: Option<String>,
) -> Vec<Effect> {
    let selected = match selected_for_operation(state, kind) {
        Ok(selected) => selected,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let active = state.workspace.active_panel;
    let other = active.other();
    let active_panel = state.workspace.panel(active);
    let other_panel = state.workspace.panel(other);
    let (operation, active_preferred) = match selected_operation(
        kind,
        selected.path,
        &active_panel.directory,
        &other_panel.directory,
        argument,
    ) {
        Ok(operation) => operation,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let refresh = refresh_requests(
        state,
        Some(RefreshSelection {
            preferred: active_preferred,
            fallback_index: active_panel.selected,
        }),
        Some(RefreshSelection {
            preferred: other_panel.selected_path().map(ToOwned::to_owned),
            fallback_index: other_panel.selected,
        }),
    );
    let request = FileOperationRequest {
        operation,
        protected_directory: other_panel.directory.clone(),
        refresh,
    };
    begin_operation(state, request)
}

fn batch_operation_from_targets(state: &mut AppState, kind: FileOperationKind) -> Vec<Effect> {
    debug_assert!(matches!(
        kind,
        FileOperationKind::Copy | FileOperationKind::Move
    ));
    let targets = match targets_for_operation(state, kind) {
        Ok(targets) => targets,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let active = state.workspace.active_panel;
    let other = active.other();
    let active_panel = state.workspace.panel(active);
    let other_panel = state.workspace.panel(other);
    let selected_path = active_panel.selected_path().map(ToOwned::to_owned);
    let moving_selected = kind == FileOperationKind::Move
        && selected_path
            .as_ref()
            .is_some_and(|path| targets.iter().any(|entry| entry.path == *path));
    let operations = match transfer_operations(kind, targets, &other_panel.directory) {
        Ok(operations) => operations,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let refresh = refresh_requests(
        state,
        Some(RefreshSelection {
            preferred: if moving_selected { None } else { selected_path },
            fallback_index: active_panel.selected,
        }),
        Some(RefreshSelection {
            preferred: other_panel.selected_path().map(ToOwned::to_owned),
            fallback_index: other_panel.selected,
        }),
    );
    begin_batch_operation(
        state,
        FileOperationBatchRequest {
            operations,
            source_panel: active,
            protected_directory: other_panel.directory.clone(),
            refresh,
        },
    )
}

pub(super) fn targets_for_operation(
    state: &AppState,
    kind: FileOperationKind,
) -> Result<Vec<crate::workspace::DirectoryEntry>, String> {
    let panel = state.workspace.active();
    let visual_range = state
        .visual_selection
        .range(state.workspace.active_panel, panel.selected);
    if panel.marked.is_empty() && visual_range.is_none() {
        return selected_for_operation(state, kind).map(|entry| vec![entry]);
    }

    let targets = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(index, entry)| {
            !entry.is_parent
                && (panel.is_marked(&entry.path)
                    || visual_range
                        .as_ref()
                        .is_some_and(|range| range.contains(index)))
        })
        .map(|(_, entry)| entry)
        .cloned()
        .collect::<Vec<_>>();
    if targets.is_empty() {
        Err(format!(
            "Cannot {:?}: the active panel has no marked entries",
            kind
        ))
    } else {
        Ok(targets)
    }
}

fn create_entry(state: &mut AppState, path: String, directory: bool) -> Vec<Effect> {
    let active = state.workspace.active_panel;
    let panel = state.workspace.active();
    let destination = normalize_from(&panel.directory, std::path::Path::new(&path));
    let preferred = (destination.parent() == Some(panel.directory.as_path()))
        .then(|| destination.clone())
        .or_else(|| panel.selected_path().map(ToOwned::to_owned));

    let operation = if directory {
        FileOperation::CreateDirectory {
            destination: destination.clone(),
        }
    } else {
        FileOperation::CreateFile {
            destination: destination.clone(),
        }
    };

    let request = FileOperationRequest {
        operation,
        protected_directory: state.workspace.panel(active.other()).directory.clone(),
        refresh: refresh_requests(
            state,
            Some(RefreshSelection {
                preferred,
                fallback_index: panel.selected,
            }),
            None,
        ),
    };
    begin_operation(state, request)
}

pub(super) fn selected_for_operation(
    state: &AppState,
    kind: FileOperationKind,
) -> Result<crate::workspace::DirectoryEntry, String> {
    let entry = state
        .selected_entry()
        .ok_or_else(|| format!("Cannot {:?}: the active panel has no selected entry", kind))?;
    if entry.is_parent {
        return Err(format!("Cannot {:?} the parent '..' entry", kind));
    }

    Ok(entry.clone())
}

fn selected_operation(
    kind: FileOperationKind,
    source: std::path::PathBuf,
    source_directory: &std::path::Path,
    destination_directory: &std::path::Path,
    argument: Option<String>,
) -> Result<(FileOperation, Option<std::path::PathBuf>), String> {
    Ok(match kind {
        FileOperationKind::Copy => {
            let Some(name) = source.file_name().map(ToOwned::to_owned) else {
                return Err(format!(
                    "Cannot copy filesystem root '{}'",
                    source.display()
                ));
            };

            (
                FileOperation::Copy {
                    source: source.clone(),
                    destination: destination_directory.join(&name),
                    overwrite: false,
                },
                Some(source),
            )
        }
        FileOperationKind::Move => {
            let Some(name) = source.file_name().map(ToOwned::to_owned) else {
                return Err(format!(
                    "Cannot move filesystem root '{}'",
                    source.display()
                ));
            };

            (
                FileOperation::Move {
                    source,
                    destination: destination_directory.join(&name),
                    overwrite: false,
                },
                None,
            )
        }
        FileOperationKind::Delete => (FileOperation::Delete { source }, None),
        FileOperationKind::Rename => {
            let Some(new_name) = argument else {
                return Err("rename usage: :rename <new-name>".into());
            };

            let destination = source_directory.join(new_name);
            (
                FileOperation::Rename {
                    source,
                    destination: destination.clone(),
                },
                Some(destination),
            )
        }
        FileOperationKind::CreateDirectory
        | FileOperationKind::CreateFile
        | FileOperationKind::CreateArchive
        | FileOperationKind::ExtractArchive => {
            unreachable!("create operations do not use a selected entry")
        }
    })
}

fn transfer_operations(
    kind: FileOperationKind,
    targets: Vec<crate::workspace::DirectoryEntry>,
    destination_directory: &std::path::Path,
) -> Result<Vec<FileOperation>, String> {
    let mut operations = Vec::with_capacity(targets.len());
    for entry in targets {
        let source = entry.path;
        let Some(name) = source.file_name().map(ToOwned::to_owned) else {
            return Err(format!(
                "Cannot {} filesystem root '{}'",
                if kind == FileOperationKind::Copy {
                    "copy"
                } else {
                    "move"
                },
                source.display()
            ));
        };

        let destination = destination_directory.join(name);
        operations.push(if kind == FileOperationKind::Copy {
            FileOperation::Copy {
                source,
                destination,
                overwrite: false,
            }
        } else {
            FileOperation::Move {
                source,
                destination,
                overwrite: false,
            }
        });
    }

    Ok(operations)
}
