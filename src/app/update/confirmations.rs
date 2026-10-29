use super::directory_edit::{
    cancel_directory_edit_exit, discard_directory_edit, save_directory_edit,
};
use super::file_operations::begin_batch_operation;
use super::file_requests::targets_for_operation;
use super::preview::prepare_selected;
use super::refresh::refresh_requests;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::dialogs::{DialogKind, FileConflict};
use crate::file_operations::{FileOperation, FileOperationBatchRequest, FileOperationKind};
use std::fs;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::RequestDelete => request_delete(state),
        Action::AcceptConfirmation => {
            if state.directory_edit.active && state.directory_edit.awaiting_confirmation {
                return save_directory_edit(state);
            }

            if state.file_conflict.is_some() {
                return resolve_file_conflict(state, true);
            }

            state.mode = AppMode::Normal;
            let Some(request) = state.confirmation.take() else {
                return Vec::new();
            };

            begin_batch_operation(state, request)
        }
        Action::RejectConfirmation => {
            if state.directory_edit.active && state.directory_edit.awaiting_confirmation {
                discard_directory_edit(state)
            } else if state.file_conflict.is_some() {
                resolve_file_conflict(state, false)
            } else {
                cancel_delete(state)
            }
        }
        Action::OverwriteFileConflict => resolve_file_conflict(state, true),
        Action::SkipFileConflict => resolve_file_conflict(state, false),
        Action::ToggleFileConflictApplyToAll => {
            if let Some(conflict) = &mut state.file_conflict {
                conflict.toggle_apply_to_all();
            }
            Vec::new()
        }
        Action::CancelConfirmation => {
            if state.directory_edit.active && state.directory_edit.awaiting_confirmation {
                return cancel_directory_edit_exit(state);
            }

            if state.file_conflict.is_some() {
                return cancel_file_conflict(state);
            }

            cancel_delete(state)
        }
        _ => unreachable!("dispatcher routed an unsupported action to confirmations"),
    }
}

fn request_delete(state: &mut AppState) -> Vec<Effect> {
    let targets = match targets_for_operation(state, FileOperationKind::Delete) {
        Ok(targets) => targets,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let active = state.workspace.active_panel;
    let other = active.other();
    let request = FileOperationBatchRequest {
        operations: targets
            .into_iter()
            .map(|entry| FileOperation::Delete { source: entry.path })
            .collect(),
        source_panel: active,
        protected_directory: state.workspace.panel(other).directory.clone(),
        refresh: refresh_requests(state, None, None),
    };

    state.mode = AppMode::Confirm;
    state.dialog_kind = DialogKind::Warning;
    state.confirmation = Some(request);
    state.pending_keys.clear();
    if state.dialog_options.display == crate::dialogs::DialogDisplay::CommandLine {
        state.notification = None;
    }
    Vec::new()
}

pub(super) fn collect_file_conflicts(
    request: &FileOperationBatchRequest,
) -> Result<Vec<FileConflict>, String> {
    let mut conflicts = Vec::new();
    for operation in &request.operations {
        if operation.overwrites() {
            continue;
        }

        let Some(source) = operation.source() else {
            continue;
        };

        let Some(destination) = operation.conflict_destination() else {
            continue;
        };

        match fs::symlink_metadata(destination) {
            Ok(_) => conflicts.push(FileConflict {
                source: source.to_path_buf(),
                destination: destination.to_path_buf(),
                new_metadata: crate::filesystem::metadata::load(source)?,
                existing_metadata: crate::filesystem::metadata::load(destination)?,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Unable to inspect {:?} destination '{}': {error}",
                    operation.kind(),
                    destination.display()
                ));
            }
        }
    }

    Ok(conflicts)
}

fn resolve_file_conflict(state: &mut AppState, overwrite: bool) -> Vec<Effect> {
    let Some(mut conflict) = state.file_conflict.take() else {
        return Vec::new();
    };

    let operation = conflict.request.kind();
    if !conflict.resolve_current(overwrite) {
        state.file_conflict = Some(conflict);
        return Vec::new();
    }

    let request = conflict.into_resolved_request();
    if request.operations.is_empty() {
        state
            .workspace
            .panel_mut(request.source_panel)
            .clear_marks();
        state.visual_selection.clear();
        state.mode = AppMode::Normal;
        state.pending_keys.clear();
        state.notification = Some(Notification::info(format!(
            "{} skipped",
            conflict_operation_name(operation)
        )));
        return prepare_selected(state);
    }

    begin_batch_operation(state, request)
}

fn cancel_file_conflict(state: &mut AppState) -> Vec<Effect> {
    let operation = state
        .file_conflict
        .as_ref()
        .and_then(|conflict| conflict.request.kind());
    state.file_conflict = None;
    state.mode = if state.visual_selection.panel.is_some() {
        AppMode::Visual
    } else {
        AppMode::Normal
    };
    state.pending_keys.clear();
    state.notification = Some(Notification::info(format!(
        "{} cancelled",
        conflict_operation_name(operation)
    )));

    prepare_selected(state)
}

fn conflict_operation_name(kind: Option<FileOperationKind>) -> &'static str {
    match kind {
        Some(FileOperationKind::Move) => "Move",
        _ => "Copy",
    }
}

fn cancel_delete(state: &mut AppState) -> Vec<Effect> {
    state.mode = if state.visual_selection.panel.is_some() {
        AppMode::Visual
    } else {
        AppMode::Normal
    };
    state.confirmation = None;
    state.notification = Some(Notification::info("Delete cancelled"));
    Vec::new()
}
