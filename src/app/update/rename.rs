use super::file_requests::selected_for_operation;
use super::playback::stop_video;
use super::preview::prepare_selected;
use super::update_action;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::file_operations::{FileOperationKind, validate_new_name};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::RequestRename { force_float } => request_rename(state, force_float),
        Action::RenameInput(input) => edit_rename(state, input),
        _ => unreachable!("dispatcher routed an unsupported action to rename"),
    }
}

fn request_rename(state: &mut AppState, force_float: bool) -> Vec<Effect> {
    use crate::dialogs::{DialogDisplay, RenamePrompt};

    let selected = match selected_for_operation(state, FileOperationKind::Rename) {
        Ok(selected) => selected,
        Err(error) => {
            state.notification = Some(Notification::error(error));
            return Vec::new();
        }
    };

    let display = if force_float {
        DialogDisplay::Float
    } else {
        state.dialog_options.display
    };

    let effects = stop_video(state);
    state.rename_prompt = Some(RenamePrompt::new(selected.path, selected.name, display));
    state.mode = if display == DialogDisplay::CommandLine {
        AppMode::Command
    } else {
        AppMode::Normal
    };

    state.command_line.clear();
    state.command_completion = None;
    state.command_history.reset_navigation();
    state.pending_keys.clear();
    state.notification = None;

    effects
}

fn edit_rename(state: &mut AppState, input: crate::dialogs::RenameInput) -> Vec<Effect> {
    use crate::dialogs::RenameInput;

    let Some(prompt) = &mut state.rename_prompt else {
        return Vec::new();
    };

    match input {
        RenameInput::Submit => {
            if let Err(error) = validate_new_name(&prompt.text) {
                prompt.error = Some(error);
                return Vec::new();
            }

            let prompt = state.rename_prompt.take().unwrap();
            state.mode = AppMode::Normal;

            if state
                .selected_entry()
                .is_none_or(|entry| entry.path != prompt.source)
            {
                state.notification = Some(Notification::error(
                    "Rename selection changed; please try again",
                ));
                return prepare_selected(state);
            }

            if prompt
                .source
                .file_name()
                .is_some_and(|name| name == std::ffi::OsStr::new(&prompt.text))
            {
                return prepare_selected(state);
            }

            // Use the same typed command implementation; names are not reparsed as shell text.
            update_action(state, Action::Rename(prompt.text))
        }
        RenameInput::Cancel => {
            state.rename_prompt = None;
            state.mode = AppMode::Normal;
            prepare_selected(state)
        }
        input => {
            prompt.edit(input);
            Vec::new()
        }
    }
}
