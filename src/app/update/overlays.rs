use super::documents::start_document_request;
use super::preview::{active_preview_panel, restore_preview_media};
use crate::app::{AppState, Effect};
use crate::preview::{DocumentFormat, DocumentPageContent, LoadState, PreviewKind};

pub(super) fn is_open(state: &AppState) -> bool {
    state.command_completion.is_some()
        || state.help.is_some()
        || state.shell_output.is_some()
        || state.operation_progress.is_some()
        || state.rename_prompt.is_some()
}

pub(super) fn sync_preview(
    state: &mut AppState,
    mut effects: Vec<Effect>,
    overlay_was_open: bool,
    preview_generation: u64,
) -> Vec<Effect> {
    if is_open(state) {
        // Native image protocols draw outside Ratatui's buffer, so suppress their
        // surface while a command float covers the underlying preview.
        effects.retain(|effect| {
            !matches!(
                effect,
                Effect::PreparePreviewMedia { .. }
                    | Effect::PresentDocumentRaster { .. }
                    | Effect::StartPreviewPlayback { .. }
            )
        });

        if !overlay_was_open {
            effects.insert(0, Effect::ClearMediaSurface);
        }
    } else if overlay_was_open
        && state.preview.generation == preview_generation
        && !state.should_quit
        && !effects
            .iter()
            .any(|effect| matches!(effect, Effect::RefreshPanels { .. }))
    {
        let mut restore = restore_command_overlay_preview(state);
        restore.extend(effects);
        effects = restore;
    }

    effects
}

fn restore_command_overlay_preview(state: &mut AppState) -> Vec<Effect> {
    if matches!(&state.preview.document, LoadState::Ready(document)
        if matches!(document.content, DocumentPageContent::Raster { .. }))
        && let (Some(panel), Some(path), Some(format)) = (
            active_preview_panel(state),
            state.preview.path.clone(),
            state
                .preview
                .path
                .as_deref()
                .and_then(DocumentFormat::from_path),
        )
    {
        return vec![start_document_request(
            state,
            panel,
            path,
            format,
            state.preview.document_page,
        )];
    }

    if matches!(state.preview.media, LoadState::Loading)
        || matches!(&state.preview.media, LoadState::Ready(media)
            if matches!(media.kind, PreviewKind::Image | PreviewKind::Video))
    {
        return restore_preview_media(state);
    }

    Vec::new()
}
