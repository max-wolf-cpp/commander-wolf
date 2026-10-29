use super::playback::stop_video;
use super::preview::prepare_selected;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::preview::{PreviewKind, ViewerStatus};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::ViewerOpenSelected => {
            let Some(entry) = state
                .selected_entry()
                .cloned()
                .filter(|entry| !entry.is_directory)
            else {
                return Vec::new();
            };

            open_viewer(state, entry.path, entry.name)
        }
        Action::ViewerClose => close_viewer(state),
        Action::ViewerProbed {
            generation,
            path,
            metadata,
            layout,
        } => {
            if !state.viewer.matches(generation, &path) {
                return Vec::new();
            }

            if metadata.kind != PreviewKind::Image {
                state.viewer.close();
                state.notification = Some(Notification::error(
                    "The selected entry is not a supported image",
                ));
                return prepare_selected(state);
            }

            state.viewer.layout = Some(layout);
            state.viewer.status = ViewerStatus::Preparing;
            vec![Effect::PrepareViewerMedia {
                generation,
                path,
                metadata,
                layout,
            }]
        }
        Action::ViewerPrepared { generation, path } => {
            if state.viewer.matches(generation, &path) {
                state.viewer.status = ViewerStatus::Ready;
            }
            Vec::new()
        }
        Action::ViewerFailed {
            generation,
            path,
            message,
        } => {
            if state.viewer.matches(generation, &path) {
                state.viewer.close();
                state.notification = Some(Notification::error(message));
                prepare_selected(state)
            } else {
                Vec::new()
            }
        }
        _ => unreachable!("dispatcher routed an unsupported action to viewer"),
    }
}

pub(super) fn open_viewer(
    state: &mut AppState,
    path: std::path::PathBuf,
    title: String,
) -> Vec<Effect> {
    if state.finder.is_open() || state.confirmation.is_some() || state.mode != AppMode::Normal {
        return Vec::new();
    }

    let mut effects = stop_video(state);
    effects.push(Effect::ClearMediaSurface);

    let generation = state.viewer.open(path.clone(), title.clone());
    state.pending_keys.clear();

    effects.push(Effect::ProbeViewer {
        generation,
        path,
        title,
        size: state.terminal_size,
        maximum_percent: state.preview_options.viewer_max_percent,
    });
    effects
}

fn close_viewer(state: &mut AppState) -> Vec<Effect> {
    if !state.viewer.is_open() {
        return Vec::new();
    }

    state.viewer.close();
    state.pending_keys.clear();

    let mut effects = vec![Effect::ClearMediaSurface];
    effects.extend(prepare_selected(state));
    effects
}
