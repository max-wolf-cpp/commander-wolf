use super::playback::stop_video;
use super::preview::prepare_selected;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::file_operations::normalize_from;
use crate::workspace::PanelId;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::Initialize => [PanelId::Left, PanelId::Right]
            .into_iter()
            .map(|panel| Effect::LoadDirectory {
                panel,
                path: state.initial_path.clone(),
                preferred: None,
            })
            .collect(),
        Action::DirectoryLoaded {
            panel,
            directory,
            entries,
            selected,
        } => {
            state.search.clear_visuals_for(panel);
            let scroll_offset = state.workspace.scroll_offset;
            state.workspace.panel_mut(panel).replace_directory(
                directory,
                entries,
                selected,
                scroll_offset,
            );

            if panel == state.workspace.active_panel {
                prepare_selected(state)
            } else {
                Vec::new()
            }
        }
        Action::DirectoryLoadFailed { panel: _, message } => {
            state.notification = Some(Notification::error(message));
            Vec::new()
        }
        Action::ToggleActivePanel => {
            let mut effects = stop_video(state);
            effects.push(Effect::ClearMediaSurface);

            state.workspace.active_panel = state.workspace.active_panel.other();

            effects.extend(prepare_selected(state));
            effects
        }
        Action::OpenParent => {
            let panel = state.workspace.active_panel;
            let current = state.workspace.active().directory.clone();
            current
                .parent()
                .map(ToOwned::to_owned)
                .map_or_else(Vec::new, |parent| {
                    let mut effects = stop_video(state);
                    effects.push(Effect::LoadDirectory {
                        panel,
                        path: parent,
                        preferred: Some(current),
                    });
                    effects
                })
        }
        Action::OpenSelected => {
            let selected = state.workspace.active().selected_entry().cloned();
            selected
                .filter(|entry| entry.is_directory)
                .map_or_else(Vec::new, |entry| open_selected_directory(state, entry))
        }
        Action::ChangeDirectory(path) => {
            let panel = state.workspace.active_panel;
            let path = normalize_from(
                &state.workspace.active().directory,
                std::path::Path::new(&path),
            );
            vec![Effect::LoadDirectory {
                panel,
                path,
                preferred: None,
            }]
        }
        Action::ToggleHidden => {
            let mut effects = stop_video(state);

            state.workspace.show_hidden = !state.workspace.show_hidden;
            let show = state.workspace.show_hidden;
            let scroll_offset = state.workspace.scroll_offset;
            for panel in &mut state.workspace.panels {
                panel.set_show_hidden(show, scroll_offset);
            }

            state.search.clear_visuals_for(PanelId::Left);
            state.search.clear_visuals_for(PanelId::Right);
            state.visual_selection.clear();
            state.mode = AppMode::Normal;
            state.pending_keys.clear();

            effects.extend(prepare_selected(state));
            effects
        }
        _ => unreachable!("dispatcher routed an unsupported action to workspace"),
    }
}

pub(super) fn open_selected_directory(
    state: &mut AppState,
    entry: crate::workspace::DirectoryEntry,
) -> Vec<Effect> {
    let panel = state.workspace.active_panel;
    let current = state.workspace.active().directory.clone();

    let mut effects = stop_video(state);
    effects.push(Effect::LoadDirectory {
        panel,
        path: entry.path,
        preferred: entry.is_parent.then_some(current),
    });
    effects
}
