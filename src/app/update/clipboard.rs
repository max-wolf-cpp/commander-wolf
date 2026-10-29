use crate::app::{Action, AppState, Effect, Notification};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    let Some(path) = state.selected_path() else {
        state.notification = Some(Notification::error("No entry selected to copy"));
        return Vec::new();
    };

    let effect = match action {
        Action::CopySelectedPath => Effect::CopyPathToClipboard(path.to_owned()),
        Action::CopySelectedName => Effect::CopyNameToClipboard(path.to_owned()),
        _ => unreachable!("dispatcher routed an unsupported action to clipboard"),
    };

    state.notification = None;
    vec![effect]
}
