use super::playback::stop_video;
use super::preview::restore_preview_media;
use crate::app::{Action, AppMode, AppState, Effect};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::EnterCommandMode => {
            let mut effects = stop_video(state);
            if !effects.is_empty() {
                effects.push(Effect::ClearMediaSurface);
                effects.extend(restore_preview_media(state));
            }

            state.mode = AppMode::Command;
            state.command_line = ":".to_owned();
            state.command_completion = None;
            state.command_history.reset_navigation();
            state.notification = None;

            effects
        }
        Action::CancelCommandMode => {
            if let Some(completion) = state.command_completion.take() {
                state.command_line = completion.original;
                return Vec::new();
            }

            state.mode = AppMode::Normal;
            state.command_line.clear();
            state.command_history.reset_navigation();
            Vec::new()
        }
        Action::PushCommandCharacter(character) => {
            state.command_completion = None;
            state.command_line.push(character);
            Vec::new()
        }
        Action::DeleteCommandCharacter => {
            state.command_completion = None;
            state.command_line.pop();
            if state.command_line.is_empty() {
                state.mode = AppMode::Normal;
                state.command_history.reset_navigation();
            }
            Vec::new()
        }
        Action::SubmitCommand => {
            state.command_completion = None;
            let line = std::mem::take(&mut state.command_line);
            state.command_history.record(&line);
            state.mode = AppMode::Normal;
            vec![Effect::ExecuteCommandLine(line)]
        }
        Action::PreviousCommandHistory => {
            state.command_completion = None;
            if state.mode == AppMode::Command
                && let Some(line) = state.command_history.previous(&state.command_line)
            {
                state.command_line = line;
            }
            Vec::new()
        }
        Action::NextCommandHistory => {
            state.command_completion = None;
            if state.mode == AppMode::Command
                && let Some(line) = state.command_history.next_entry()
            {
                state.command_line = line;
            }
            Vec::new()
        }
        Action::CompleteCommand { backwards } => {
            if state.mode != AppMode::Command {
                return Vec::new();
            }

            if let Some(completion) = &mut state.command_completion {
                completion.cycle(backwards);
                state.command_line = completion.line();
                Vec::new()
            } else {
                vec![Effect::CompleteCommand {
                    line: state.command_line.clone(),
                    backwards,
                }]
            }
        }
        Action::CommandCompletionsLoaded {
            line,
            items,
            backwards,
        } => {
            if state.mode == AppMode::Command && state.command_line == line {
                state.command_completion =
                    crate::commands::completion::CommandCompletion::new(line, items, backwards);
                if let Some(completion) = &state.command_completion {
                    state.command_line = completion.line();
                }
            }
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to command_line"),
    }
}
