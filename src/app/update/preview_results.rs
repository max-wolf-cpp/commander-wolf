use super::preview::{clamp_metadata_viewport, metadata_content_rows};
use crate::app::{Action, AppState, Effect};
use crate::preview::{LoadState, PreviewKind};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::PreviewMetadataPageDown => {
            if state.workspace.preview_visible {
                let row_count = metadata_content_rows(state);
                state.preview.metadata_viewport.page_down(row_count);
            }
            Vec::new()
        }
        Action::PreviewMetadataPageUp => {
            if state.workspace.preview_visible {
                state.preview.metadata_viewport.page_up();
            }
            Vec::new()
        }
        Action::PreviewMetadataLoaded {
            generation,
            path,
            metadata,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.metadata = LoadState::Ready(metadata);
                clamp_metadata_viewport(state);
            }
            Vec::new()
        }
        Action::PreviewMetadataFailed {
            generation,
            path,
            message,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.metadata = LoadState::Unavailable(message);
                clamp_metadata_viewport(state);
            }
            Vec::new()
        }
        Action::PreviewMediaPrepared {
            generation,
            path,
            metadata,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = metadata.kind;
                state.preview.has_audio = metadata.has_audio;
                state.preview.media = LoadState::Ready(metadata);
                state.preview.surface_suspended = false;
                clamp_metadata_viewport(state);
            }
            Vec::new()
        }
        Action::PreviewMediaUnavailable {
            generation,
            path,
            message: _,
        } => {
            if !state.preview.matches(generation, &path) {
                return Vec::new();
            }

            state.preview.kind = PreviewKind::None;
            state.preview.media = LoadState::Idle;

            if crate::preview::ArchiveFormat::from_path(&path).is_some() {
                state.preview.archive = LoadState::Loading;
                return vec![Effect::LoadPreviewArchive { generation, path }];
            }

            state.preview.text = LoadState::Loading;
            clamp_metadata_viewport(state);
            vec![Effect::LoadPreviewText { generation, path }]
        }
        Action::PreviewTextLoaded {
            generation,
            path,
            preview,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::Text;
                state.preview.text = LoadState::Ready(preview);
            }
            Vec::new()
        }
        Action::PreviewTextUnavailable {
            generation,
            path,
            message,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::None;
                state.preview.text = LoadState::Unavailable(message);
            }
            Vec::new()
        }
        Action::PreviewDirectoryLoaded {
            generation,
            path,
            preview,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::Directory;
                state.preview.directory = LoadState::Ready(preview);
            }
            Vec::new()
        }
        Action::PreviewDirectoryFailed {
            generation,
            path,
            message,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::Directory;
                state.preview.directory = LoadState::Unavailable(message);
            }
            Vec::new()
        }
        Action::PreviewArchiveLoaded {
            generation,
            path,
            preview,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::Archive;
                state.preview.archive = LoadState::Ready(preview);
            }
            Vec::new()
        }
        Action::PreviewArchiveFailed {
            generation,
            path,
            message,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = PreviewKind::Archive;
                state.preview.archive = LoadState::Unavailable(message);
            }
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to preview_results"),
    }
}
