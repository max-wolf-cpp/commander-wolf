use super::preview::{active_preview_panel, clamp_metadata_viewport, preview_media_region};
use crate::app::{Action, AppState, Effect, Notification};
use crate::preview::{
    DocumentFormat, DocumentPageContent, DocumentRequest, LoadState, PreviewKind,
};
use crate::workspace::PanelId;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::PreviewPageNext { count } => {
            navigate_document(state, count.clamp(1, isize::MAX as usize) as isize)
        }
        Action::PreviewPagePrevious { count } => {
            navigate_document(state, -(count.clamp(1, isize::MAX as usize) as isize))
        }
        Action::PreviewDocumentLoaded {
            generation,
            request_id,
            path,
            preview,
            raster,
        } => {
            if !state.preview.matches(generation, &path)
                || state.preview.document_request_id != request_id
            {
                return Vec::new();
            }

            state.preview.kind = PreviewKind::Document;
            state.preview.media = LoadState::Idle;
            state.preview.document_page = preview.current_page;
            state.preview.document = LoadState::Ready(preview);
            clamp_metadata_viewport(state);

            match (raster, active_preview_panel(state)) {
                (Some(raster), Some(panel)) => vec![Effect::PresentDocumentRaster {
                    generation,
                    request_id,
                    path,
                    panel,
                    region: preview_media_region(state, panel),
                    raster,
                }],
                _ => Vec::new(),
            }
        }
        Action::PreviewDocumentFailed {
            generation,
            request_id,
            path,
            message,
        } => {
            if state.preview.matches(generation, &path)
                && state.preview.document_request_id == request_id
            {
                state.preview.kind = PreviewKind::Document;
                state.preview.media = LoadState::Idle;
                state.preview.document = LoadState::Unavailable(message);
                clamp_metadata_viewport(state);
            }
            Vec::new()
        }
        Action::PreviewDocumentRasterPresented {
            generation,
            request_id,
            path,
        } => {
            if state.preview.matches(generation, &path)
                && state.preview.document_request_id == request_id
                && let LoadState::Ready(document) = &mut state.preview.document
                && let DocumentPageContent::Raster { surface_ready, .. } = &mut document.content
            {
                *surface_ready = true;
                state.preview.surface_suspended = false;
            }
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to documents"),
    }
}

pub(super) fn start_document_request(
    state: &mut AppState,
    panel: PanelId,
    path: std::path::PathBuf,
    format: DocumentFormat,
    page: usize,
) -> Effect {
    state.preview.document_request_id = state.preview.document_request_id.wrapping_add(1).max(1);
    state.preview.document_page = page;
    state.preview.document = LoadState::Loading;

    let region = preview_media_region(state, panel);
    Effect::StartDocumentPreview(DocumentRequest {
        generation: state.preview.generation,
        request_id: state.preview.document_request_id,
        path,
        format,
        page,
        pixel_width: 1,
        pixel_height: 1,
        text_columns: if format.is_book() {
            region.columns.saturating_sub(2).max(1) as u16
        } else {
            region.columns.max(1) as u16
        },
        text_rows: region.rows.max(1) as u16,
        options: state.preview_options.documents,
    })
}

fn navigate_document(state: &mut AppState, delta: isize) -> Vec<Effect> {
    let LoadState::Ready(document) = &state.preview.document else {
        state.notification = Some(Notification::info("Current preview is not a document"));
        return Vec::new();
    };

    if document.total_pages <= 1 {
        state.notification = Some(Notification::info("Document has only one page"));
        return Vec::new();
    }

    let current = document.current_page;
    let total = document.total_pages;
    let target = current.saturating_add_signed(delta).min(total - 1);
    if target == current {
        state.notification = Some(Notification::info(if delta < 0 {
            "Already at the first document page"
        } else {
            "Already at the last document page"
        }));
        return Vec::new();
    }

    let (Some(panel), Some(path), Some(format)) = (
        active_preview_panel(state),
        state.preview.path.clone(),
        state
            .preview
            .path
            .as_deref()
            .and_then(DocumentFormat::from_path),
    ) else {
        return Vec::new();
    };

    vec![
        Effect::CancelDocumentPreview,
        Effect::ClearMediaSurface,
        start_document_request(state, panel, path, format, target),
    ]
}
