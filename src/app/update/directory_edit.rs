use super::playback::stop_video;
use super::preview::{clear_preview, prepare_selected};
use super::refresh::{apply_refresh_results, refresh_requests};
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::dialogs::DialogKind;
use crate::file_operations::{FileOperationKind, OperationStatus, RefreshContext};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::EditBegin => begin_directory_edit(state),
        Action::EditInsertCharacter(character) => {
            state.directory_edit.insert(character);
            Vec::new()
        }
        Action::EditNewline => {
            state.directory_edit.newline();
            Vec::new()
        }
        Action::EditBackspace => {
            state.directory_edit.backspace();
            Vec::new()
        }
        Action::EditDelete => {
            state.directory_edit.delete();
            Vec::new()
        }
        Action::EditMoveLeft => {
            state.directory_edit.move_left();
            Vec::new()
        }
        Action::EditMoveRight => {
            state.directory_edit.move_right();
            Vec::new()
        }
        Action::EditMoveUp => {
            state.directory_edit.move_up();
            Vec::new()
        }
        Action::EditMoveDown => {
            state.directory_edit.move_down();
            Vec::new()
        }
        Action::EditHome => {
            state.directory_edit.home();
            Vec::new()
        }
        Action::EditEnd => {
            state.directory_edit.end();
            Vec::new()
        }
        Action::EditRequestExit => request_directory_edit_exit(state),
        Action::EditDiscard => discard_directory_edit(state),
        Action::EditCancelExit => cancel_directory_edit_exit(state),
        Action::DirectoryEditCompleted {
            request: _,
            outcome,
            results,
        } => {
            state.operation_progress = None;
            complete_directory_edit(state, outcome, *results)
        }
        _ => unreachable!("dispatcher routed an unsupported action to directory_edit"),
    }
}

fn begin_directory_edit(state: &mut AppState) -> Vec<Effect> {
    if state.mode != AppMode::Normal
        || state.finder.is_open()
        || state.directory_tree.open
        || state.viewer.is_open()
    {
        return Vec::new();
    }

    let panel = state.workspace.active_panel;
    let panel_state = state.workspace.active();
    state.directory_edit.begin(
        panel,
        panel_state.directory.clone(),
        &panel_state.entries,
        panel_state.selected,
    );
    state
        .directory_edit
        .set_visible_rows(panel_state.viewport.visible_rows);
    state.mode = AppMode::Insert;
    state.notification = None;
    state.pending_keys.clear();

    let mut effects = stop_video(state);
    effects.extend(clear_preview(state));
    effects
}

fn request_directory_edit_exit(state: &mut AppState) -> Vec<Effect> {
    if state.mode != AppMode::Insert || !state.directory_edit.active {
        return Vec::new();
    }

    if !state.directory_edit.has_changes() {
        state.directory_edit.clear();
        state.mode = AppMode::Normal;
        state.pending_keys.clear();
        return prepare_selected(state);
    }

    state.mode = AppMode::Confirm;
    state.dialog_kind = DialogKind::Info;
    state.directory_edit.awaiting_confirmation = true;
    state.pending_keys.clear();
    if state.dialog_options.display == crate::dialogs::DialogDisplay::CommandLine {
        state.notification = None;
    }

    Vec::new()
}

pub(super) fn cancel_directory_edit_exit(state: &mut AppState) -> Vec<Effect> {
    if state.directory_edit.active {
        state.directory_edit.awaiting_confirmation = false;
        state.mode = AppMode::Insert;
        state.notification = None;
    }
    Vec::new()
}

pub(super) fn discard_directory_edit(state: &mut AppState) -> Vec<Effect> {
    if !state.directory_edit.active {
        return Vec::new();
    }

    state.directory_edit.clear();
    state.mode = AppMode::Normal;
    state.notification = Some(Notification::info("Directory edits discarded"));
    prepare_selected(state)
}

pub(super) fn save_directory_edit(state: &mut AppState) -> Vec<Effect> {
    let other_directory = state
        .workspace
        .panel(state.directory_edit.panel.other())
        .directory
        .clone();
    let refresh = refresh_requests(state, None, None);
    let request = match state.directory_edit.request(other_directory, refresh) {
        Ok(request) => request,
        Err(error) => {
            state.directory_edit.awaiting_confirmation = false;
            state.mode = AppMode::Insert;
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    state.directory_edit.awaiting_confirmation = false;

    vec![Effect::ApplyDirectoryEdit(request)]
}

fn complete_directory_edit(
    state: &mut AppState,
    outcome: crate::file_operations::FileOperationOutcome,
    results: [crate::file_operations::PanelRefreshResult; 2],
) -> Vec<Effect> {
    if outcome.status == OperationStatus::Failure && !outcome.filesystem_changed {
        state.mode = AppMode::Insert;
        state.notification = Some(Notification::error(outcome.message));
        return Vec::new();
    }

    state.directory_edit.clear();
    state.mode = AppMode::Normal;

    let notification = if outcome.status == OperationStatus::Success {
        Notification::info(outcome.message.clone())
    } else {
        Notification::error(outcome.message.clone())
    };
    let effects = apply_refresh_results(
        state,
        results,
        RefreshContext::Operation {
            kind: FileOperationKind::Rename,
            outcome,
        },
    );
    state.notification = Some(notification);
    effects
}
