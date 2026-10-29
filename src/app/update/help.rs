use crate::app::{Action, AppState, Effect};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::OpenHelp => vec![Effect::LoadHelp],
        Action::HelpLoaded(help) => {
            state.help = Some(help);
            Vec::new()
        }
        Action::ScrollHelp(delta) => {
            if let Some(help) = &mut state.help {
                help.scroll_by(
                    delta,
                    crate::ui::help_visible_rows(state.terminal_size.area()),
                );
            }
            Vec::new()
        }
        Action::CloseHelp => {
            state.help = None;
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to help"),
    }
}
