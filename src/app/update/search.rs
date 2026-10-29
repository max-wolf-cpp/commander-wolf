use super::playback::stop_video;
use super::preview::prepare_selected;
use crate::app::{Action, AppMode, AppState, Effect};
use crate::search::{SearchDirection, repeat_match};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::SearchBegin => begin_search(state),
        Action::SearchPushCharacter(character) => {
            state.search.query.push(character);
            update_search_draft(state)
        }
        Action::SearchBackspace => {
            if state.search.query.pop().is_some() {
                update_search_draft(state)
            } else {
                cancel_search(state)
            }
        }
        Action::SearchSubmit => {
            state.search.submit();
            Vec::new()
        }
        Action::SearchCancel => cancel_search(state),
        Action::SearchRepeat { direction, count } => repeat_search(state, direction, count),
        _ => unreachable!("dispatcher routed an unsupported action to search"),
    }
}

fn begin_search(state: &mut AppState) -> Vec<Effect> {
    if !matches!(state.mode, AppMode::Normal | AppMode::Search) || state.search.active {
        return Vec::new();
    }

    let panel = state.workspace.active_panel;
    let selected = state.workspace.active().selected;
    state.search.begin(panel, selected);
    state.mode = AppMode::Search;
    state.notification = None;
    state.pending_keys.clear();

    let mut effects = stop_video(state);
    if !effects.is_empty() {
        effects.extend(prepare_selected(state));
    }
    effects
}

fn update_search_draft(state: &mut AppState) -> Vec<Effect> {
    let Some(panel) = state.search.panel else {
        return Vec::new();
    };

    let old = state.workspace.panel(panel).selected;
    let selected = state
        .search
        .update_draft(&state.workspace.panel(panel).entries);

    let scroll_offset = state.workspace.scroll_offset;
    let panel_state = state.workspace.panel_mut(panel);
    panel_state.selected = selected.min(panel_state.entries.len().saturating_sub(1));
    panel_state.viewport.ensure_visible(
        panel_state.selected,
        panel_state.entries.len(),
        scroll_offset,
    );

    if old != panel_state.selected && panel == state.workspace.active_panel {
        prepare_selected(state)
    } else {
        Vec::new()
    }
}

fn cancel_search(state: &mut AppState) -> Vec<Effect> {
    if state.mode != AppMode::Search || !state.search.active {
        state.search.dismiss_confirmed();
        if state.mode == AppMode::Search {
            state.mode = AppMode::Normal;
        }
        state.pending_keys.clear();
        return Vec::new();
    }

    let panel = state.search.panel;
    let original = state.search.cancel();
    state.mode = AppMode::Normal;
    if let Some(panel) = panel {
        let scroll_offset = state.workspace.scroll_offset;
        let panel_state = state.workspace.panel_mut(panel);
        panel_state.selected = original.min(panel_state.entries.len().saturating_sub(1));
        panel_state.viewport.ensure_visible(
            panel_state.selected,
            panel_state.entries.len(),
            scroll_offset,
        );
    }

    prepare_selected(state)
}

fn repeat_search(state: &mut AppState, direction: SearchDirection, count: usize) -> Vec<Effect> {
    if !matches!(state.mode, AppMode::Normal | AppMode::Search)
        || state.search.active
        || !state.search.confirmed_active
    {
        return Vec::new();
    }

    let Some(query) = state.search.last_confirmed_query.clone() else {
        return Vec::new();
    };

    let panel = state.workspace.active_panel;
    let panel_state = state.workspace.active();
    let Some(selected) = repeat_match(
        &panel_state.entries,
        &query,
        panel_state.selected,
        direction,
        count,
    ) else {
        state.search.no_match = true;
        return Vec::new();
    };

    let scroll_offset = state.workspace.scroll_offset;
    let panel_state = state.workspace.active_mut();
    panel_state.selected = selected;
    panel_state
        .viewport
        .ensure_visible(selected, panel_state.entries.len(), scroll_offset);

    state.search.highlight_panel = Some(panel);
    state.search.highlight_query = Some(query);
    state.search.current_match = Some(selected);
    state.search.no_match = false;

    prepare_selected(state)
}
