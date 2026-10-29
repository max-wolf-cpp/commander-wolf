use super::playback::stop_video;
use super::preview::prepare_selected;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::workspace::SelectionMotion;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::ToggleMark => toggle_mark(state),
        Action::VisualBegin => begin_visual_selection(state),
        Action::VisualCancel => cancel_visual_selection(state),
        Action::MoveSelection { delta, count } => {
            let motion = if delta < 0 {
                SelectionMotion::Previous
            } else {
                SelectionMotion::Next
            };
            move_selection(state, motion, Some(count))
        }
        Action::Select { motion, count } => move_selection(state, motion, count),
        Action::CenterSelection => {
            state.workspace.active_mut().center_selection();
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to selection"),
    }
}

fn toggle_mark(state: &mut AppState) -> Vec<Effect> {
    let Some(entry) = state.workspace.active().selected_entry().cloned() else {
        state.notification = Some(Notification::error(
            "Cannot toggle a mark: the active panel has no selected entry",
        ));
        return Vec::new();
    };

    if entry.is_parent {
        state.notification = Some(Notification::error("Cannot mark the parent '..' entry"));
        return Vec::new();
    }

    state.workspace.active_mut().toggle_mark(&entry.path);
    state.notification = None;
    Vec::new()
}

fn begin_visual_selection(state: &mut AppState) -> Vec<Effect> {
    if state.mode != AppMode::Normal {
        return Vec::new();
    }

    let Some(entry) = state.workspace.active().selected_entry() else {
        state.notification = Some(Notification::error(
            "Cannot begin visual selection: the active panel is empty",
        ));
        return Vec::new();
    };

    if entry.is_parent {
        state.notification = Some(Notification::error(
            "Cannot begin visual selection on the parent '..' entry",
        ));
        return Vec::new();
    }

    state.visual_selection.begin(
        state.workspace.active_panel,
        state.workspace.active().selected,
    );
    state.mode = AppMode::Visual;
    state.notification = None;
    state.pending_keys.clear();

    Vec::new()
}

fn cancel_visual_selection(state: &mut AppState) -> Vec<Effect> {
    if state.visual_selection.panel.is_none() {
        return Vec::new();
    }

    state.visual_selection.clear();
    state.mode = AppMode::Normal;
    state.pending_keys.clear();
    Vec::new()
}

fn move_selection(
    state: &mut AppState,
    motion: SelectionMotion,
    count: Option<usize>,
) -> Vec<Effect> {
    let selected = state.workspace.active().selected;
    let scroll_offset = state.workspace.scroll_offset;
    state
        .workspace
        .active_mut()
        .apply_motion(motion, count, scroll_offset);

    if state.workspace.active().selected == selected {
        return Vec::new();
    }

    let mut effects = stop_video(state);
    effects.extend(prepare_selected(state));
    effects
}
