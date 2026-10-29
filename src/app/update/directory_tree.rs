use super::playback::stop_video;
use super::preview::{clear_preview, prepare_selected};
use crate::app::{Action, AppMode, AppState, Effect};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::TreeToggle => toggle_tree(state),
        Action::TreeClose => close_tree(state),
        Action::TreeSelect { motion, count } => {
            state.directory_tree.apply_motion(motion, count);
            Vec::new()
        }
        Action::TreeCenterSelection => {
            state.directory_tree.center_selection();
            Vec::new()
        }
        Action::TreeToggleSelected => toggle_tree_selected(state),
        Action::TreeExpandSelected => expand_tree_selected(state),
        Action::TreeCollapseSelected => {
            if state
                .directory_tree
                .selected_entry()
                .is_some_and(|entry| entry.depth == 0)
            {
                return Vec::new();
            }

            state.directory_tree.collapse_selected();
            Vec::new()
        }
        Action::TreeExpandAll => expand_tree_all(state),
        Action::TreeCollapseAll => {
            state.directory_tree.generation =
                state.directory_tree.generation.wrapping_add(1).max(1);
            state.directory_tree.collapse_all();
            Vec::new()
        }
        Action::TreeOpenSelected => open_tree_selected(state),
        Action::TreeChildrenLoaded {
            generation,
            parent,
            entries,
        } => {
            if state.directory_tree.open && generation == state.directory_tree.generation {
                state.directory_tree.replace_children(&parent, entries);
            }
            Vec::new()
        }
        Action::TreeExpandedAll {
            generation,
            entries,
        } => {
            if state.directory_tree.open && generation == state.directory_tree.generation {
                state.directory_tree.replace_all(entries);
            }
            Vec::new()
        }
        Action::TreeLoadFailed {
            generation,
            message,
        } => {
            if state.directory_tree.open && generation == state.directory_tree.generation {
                state.directory_tree.error = Some(message);
                if let Some(entry) = state
                    .directory_tree
                    .entries
                    .iter_mut()
                    .find(|entry| entry.loading)
                {
                    entry.loading = false;
                    entry.expanded = false;
                }
            }
            Vec::new()
        }
        Action::TreeRevealLoaded {
            generation,
            panel,
            directory,
            entries,
            selected,
        } => {
            if !state.directory_tree.open
                || generation != state.directory_tree.generation
                || panel != state.directory_tree.panel
            {
                return Vec::new();
            }

            let scroll_offset = state.workspace.scroll_offset;
            state.workspace.panel_mut(panel).replace_directory(
                directory,
                entries,
                selected,
                scroll_offset,
            );
            state.search.clear_visuals_for(panel);
            close_tree(state)
        }
        _ => unreachable!("dispatcher routed an unsupported action to directory_tree"),
    }
}

fn toggle_tree(state: &mut AppState) -> Vec<Effect> {
    if state.directory_tree.open {
        return close_tree(state);
    }

    if state.mode != AppMode::Normal || state.finder.is_open() || state.viewer.is_open() {
        return Vec::new();
    }

    let panel = state.workspace.active_panel;
    let root = state.workspace.active().directory.clone();

    let mut effects = stop_video(state);
    effects.extend(clear_preview(state));

    state.directory_tree.open(panel, root.clone());
    state
        .directory_tree
        .set_visible_rows(crate::ui::tree_visible_rows(state.terminal_size.area()));
    state.pending_keys.clear();

    effects.push(Effect::LoadTreeChildren {
        show_hidden: state.workspace.show_hidden,
        generation: state.directory_tree.generation,
        parent: root,
    });
    effects
}

fn close_tree(state: &mut AppState) -> Vec<Effect> {
    state.directory_tree.close();
    state.pending_keys.clear();

    prepare_selected(state)
}

fn expand_tree_selected(state: &mut AppState) -> Vec<Effect> {
    let Some(entry) = state.directory_tree.selected_entry().cloned() else {
        return Vec::new();
    };

    if !entry.is_directory || entry.expanded || entry.loading {
        return Vec::new();
    }

    state.directory_tree.entries[state.directory_tree.selected].loading = true;
    state.directory_tree.error = None;

    vec![Effect::LoadTreeChildren {
        show_hidden: state.workspace.show_hidden,
        generation: state.directory_tree.generation,
        parent: entry.path,
    }]
}

fn toggle_tree_selected(state: &mut AppState) -> Vec<Effect> {
    let Some(entry) = state.directory_tree.selected_entry().cloned() else {
        return Vec::new();
    };

    if !entry.is_directory {
        return Vec::new();
    }

    if entry.expanded && entry.depth > 0 {
        state.directory_tree.collapse_selected();
        Vec::new()
    } else {
        expand_tree_selected(state)
    }
}

fn expand_tree_all(state: &mut AppState) -> Vec<Effect> {
    if !state.directory_tree.open {
        return Vec::new();
    }

    state.directory_tree.generation = state.directory_tree.generation.wrapping_add(1).max(1);
    state.directory_tree.entries.truncate(1);
    state.directory_tree.entries[0].expanded = true;
    state.directory_tree.entries[0].loading = true;
    state.directory_tree.selected = 0;
    state.directory_tree.viewport.scroll = 0;
    state.directory_tree.error = None;

    vec![Effect::ExpandTreeAll {
        show_hidden: state.workspace.show_hidden,
        generation: state.directory_tree.generation,
        root: state.directory_tree.root.clone(),
    }]
}

fn open_tree_selected(state: &mut AppState) -> Vec<Effect> {
    let Some(entry) = state.directory_tree.selected_entry().cloned() else {
        return Vec::new();
    };

    vec![Effect::RevealTreeEntry {
        generation: state.directory_tree.generation,
        panel: state.directory_tree.panel,
        path: entry.path,
        is_directory: entry.is_directory,
    }]
}
