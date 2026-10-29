use super::confirmations::collect_file_conflicts;
use super::playback::stop_video;
use super::preview::{clear_preview, prepare_selected};
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::dialogs::{DialogKind, FileConflictState};
use crate::file_operations::{
    FileOperationBatchRequest, FileOperationKind, FileOperationRequest, OperationStatus,
    RefreshContext,
};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::FileOperationProgressStarted(progress) => {
            let effects = stop_video(state);

            state.operation_progress = Some(progress);
            state.mode = AppMode::Normal;
            state.pending_keys.clear();
            state.notification = None;

            effects
        }
        Action::FileOperationProgressUpdated(mut progress) => {
            if let Some(current) = &state.operation_progress {
                progress.cancelling = current.cancelling;
                state.operation_progress = Some(progress);
            }
            Vec::new()
        }
        Action::CancelFileOperation => {
            if let Some(progress) = &mut state.operation_progress
                && !progress.cancelling
            {
                progress.cancelling = true;
                vec![Effect::CancelFileOperation]
            } else {
                Vec::new()
            }
        }
        Action::FileOperationCompleted { request, outcome } => {
            let had_progress = state.operation_progress.take().is_some();
            let kind = request.operation.kind();
            complete_operation(state, kind, outcome, request.refresh, had_progress)
        }
        Action::FileOperationBatchCompleted { request, outcome } => {
            let had_progress = state.operation_progress.take().is_some();
            let Some(kind) = request.kind() else {
                state.notification = Some(Notification::error("File operation batch was empty"));
                return Vec::new();
            };

            state
                .workspace
                .panel_mut(request.source_panel)
                .clear_marks();
            complete_operation(state, kind, outcome, request.refresh, had_progress)
        }
        _ => unreachable!("dispatcher routed an unsupported action to file_operations"),
    }
}

pub(super) fn begin_operation(state: &mut AppState, request: FileOperationRequest) -> Vec<Effect> {
    let kind = request.operation.kind();
    let mut effects = stop_video(state);
    if !effects.is_empty() || is_destructive(kind) {
        effects.extend(clear_preview(state));
    }

    effects.push(Effect::PerformFileOperation(request));
    effects
}

pub(super) fn begin_batch_operation(
    state: &mut AppState,
    request: FileOperationBatchRequest,
) -> Vec<Effect> {
    let Some(kind) = request.kind() else {
        state.notification = Some(Notification::error("File operation batch is empty"));
        return Vec::new();
    };

    if matches!(kind, FileOperationKind::Copy | FileOperationKind::Move) {
        match collect_file_conflicts(&request) {
            Ok(conflicts) if !conflicts.is_empty() => {
                state.mode = AppMode::Confirm;
                state.dialog_kind = DialogKind::Warning;
                state.file_conflict = Some(FileConflictState::new(request, conflicts));
                state.pending_keys.clear();
                state.notification = None;

                let mut effects = stop_video(state);
                effects.extend(clear_preview(state));
                return effects;
            }
            Ok(_) => {}
            Err(message) => {
                state.notification = Some(Notification::error(message));
                return Vec::new();
            }
        }
    }

    let mut effects = stop_video(state);
    if !effects.is_empty() || is_destructive(kind) {
        effects.extend(clear_preview(state));
    }

    state.visual_selection.clear();
    state.mode = AppMode::Normal;
    state.pending_keys.clear();

    effects.push(Effect::PerformFileOperationBatch(request));
    effects
}

pub(super) fn is_destructive(kind: FileOperationKind) -> bool {
    matches!(
        kind,
        FileOperationKind::Move | FileOperationKind::Delete | FileOperationKind::Rename
    )
}

fn complete_operation(
    state: &mut AppState,
    kind: FileOperationKind,
    outcome: crate::file_operations::FileOperationOutcome,
    refresh: [crate::file_operations::PanelRefreshRequest; 2],
    had_progress: bool,
) -> Vec<Effect> {
    state.notification = Some(if outcome.status == OperationStatus::Success {
        Notification::info(outcome.message.clone())
    } else {
        Notification::error(outcome.message.clone())
    });

    if outcome.filesystem_changed {
        vec![Effect::RefreshPanels {
            requests: refresh,
            context: RefreshContext::Operation { kind, outcome },
        }]
    } else if is_destructive(kind) || had_progress {
        prepare_selected(state)
    } else {
        Vec::new()
    }
}
