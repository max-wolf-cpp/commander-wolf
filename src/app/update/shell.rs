use super::playback::stop_video;
use super::refresh::refresh_requests;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::file_operations::RefreshContext;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::RunShellCommand(command) => {
            if state.shell_output.is_some() {
                return Vec::new();
            }

            if command.trim().is_empty() {
                state.notification = Some(Notification::error("Usage: :!<command>"));
                return Vec::new();
            }

            let mut effects = stop_video(state);

            state.mode = AppMode::Normal;
            state.command_completion = None;
            state.command_line.clear();
            state.pending_keys.clear();
            state.notification = None;
            state.shell_output = Some(crate::shell::ShellOutput::new(command.clone()));

            effects.push(Effect::RunShellCommand {
                command,
                directory: state.workspace.active().directory.clone(),
            });
            effects
        }
        Action::ShellOutput(text) => {
            if let Some(output) = &mut state.shell_output {
                output.append(&text);
            }
            Vec::new()
        }
        Action::ShellFinished { message, failed } => {
            if let Some(output) = &mut state.shell_output {
                output.finished = Some(message);
                output.failed = failed;
            }
            Vec::new()
        }
        Action::CloseShellOutput => {
            if state
                .shell_output
                .as_ref()
                .is_some_and(|output| output.finished.is_some())
            {
                state.shell_output = None;
                vec![Effect::RefreshPanels {
                    requests: refresh_requests(state, None, None),
                    context: RefreshContext::Manual,
                }]
            } else {
                Vec::new()
            }
        }
        Action::CancelShellCommand => {
            if state
                .shell_output
                .as_ref()
                .is_some_and(|output| output.finished.is_none())
            {
                vec![Effect::CancelShellCommand]
            } else {
                Vec::new()
            }
        }
        Action::ScrollShellOutput(delta) => {
            if let Some(output) = &mut state.shell_output {
                output.scroll = output
                    .scroll
                    .saturating_add_signed(delta)
                    .min(output.lines.len().saturating_sub(1));
            }
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to shell"),
    }
}
