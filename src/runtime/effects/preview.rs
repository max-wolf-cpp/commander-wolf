use std::path::PathBuf;

use fox_media_term::Region;

use crate::app::Action;
use crate::preview::{DocumentRequest, ImageViewerLayout};
use crate::terminal::TerminalSize;
use crate::workspace::PanelId;

use super::super::document::DocumentRuntime;
use super::super::media::MediaRuntime;
use super::super::terminal_writer::TerminalWriter;

pub(super) fn load_metadata(generation: u64, path: PathBuf) -> Result<Vec<Action>, String> {
    Ok(vec![match crate::filesystem::metadata::load(&path) {
        Ok(metadata) => Action::PreviewMetadataLoaded {
            generation,
            path,
            metadata,
        },
        Err(message) => Action::PreviewMetadataFailed {
            generation,
            path,
            message,
        },
    }])
}

pub(super) fn load_text(generation: u64, path: PathBuf) -> Result<Vec<Action>, String> {
    Ok(vec![match crate::filesystem::text_preview::load(&path) {
        Ok(preview) => Action::PreviewTextLoaded {
            generation,
            path,
            preview,
        },
        Err(message) => Action::PreviewTextUnavailable {
            generation,
            path,
            message,
        },
    }])
}

pub(super) fn load_archive(generation: u64, path: PathBuf) -> Result<Vec<Action>, String> {
    Ok(vec![
        match crate::filesystem::archive_preview::load(&path) {
            Ok(preview) => Action::PreviewArchiveLoaded {
                generation,
                path,
                preview,
            },
            Err(message) => Action::PreviewArchiveFailed {
                generation,
                path,
                message,
            },
        },
    ])
}

pub(super) fn load_directory(
    generation: u64,
    path: PathBuf,
    show_hidden: bool,
) -> Result<Vec<Action>, String> {
    Ok(vec![
        match crate::filesystem::directory_preview::load(&path, show_hidden) {
            Ok(preview) => Action::PreviewDirectoryLoaded {
                generation,
                path,
                preview,
            },
            Err(message) => Action::PreviewDirectoryFailed {
                generation,
                path,
                message,
            },
        },
    ])
}

pub(super) fn prepare_media(
    generation: u64,
    path: PathBuf,
    panel: PanelId,
    region: Region,
    media: &mut MediaRuntime,
    writer: &mut TerminalWriter,
) -> Result<Vec<Action>, String> {
    Ok(vec![
        match media.prepare_preview(generation, &path, panel, region, writer) {
            Ok(action) => action,
            Err(message) => Action::PreviewMediaUnavailable {
                generation,
                path,
                message,
            },
        },
    ])
}

pub(super) fn start_document(
    mut request: DocumentRequest,
    media: &MediaRuntime,
    documents: &DocumentRuntime,
) -> Result<Vec<Action>, String> {
    let (width, height) = media.maximum_pixel_size(
        fox_media_term::Region {
            column: 0,
            row: 0,
            columns: i32::from(request.text_columns),
            rows: i32::from(request.text_rows),
        },
        request.options.max_render_pixels,
    );

    request.pixel_width = width;
    request.pixel_height = height;
    documents.start(request);

    Ok(Vec::new())
}

pub(super) fn document_raster_result(
    generation: u64,
    request_id: u64,
    path: PathBuf,
    result: Result<(), String>,
) -> Action {
    match result {
        Ok(()) => Action::PreviewDocumentRasterPresented {
            generation,
            request_id,
            path,
        },
        Err(message) => Action::PreviewDocumentFailed {
            generation,
            request_id,
            path,
            message,
        },
    }
}

pub(super) fn probe_viewer(
    generation: u64,
    path: PathBuf,
    title: String,
    size: TerminalSize,
    maximum_percent: u16,
    media: &MediaRuntime,
) -> Result<Vec<Action>, String> {
    Ok(vec![
        match media.probe_viewer(generation, &path, &title, size, maximum_percent) {
            Ok(action) => action,
            Err(message) => Action::ViewerFailed {
                generation,
                path,
                message,
            },
        },
    ])
}

pub(super) fn prepare_viewer(
    generation: u64,
    path: PathBuf,
    layout: ImageViewerLayout,
    media: &mut MediaRuntime,
    writer: &mut TerminalWriter,
) -> Result<Vec<Action>, String> {
    Ok(vec![
        match media.prepare_viewer(generation, &path, layout, writer) {
            Ok(action) => action,
            Err(message) => Action::ViewerFailed {
                generation,
                path,
                message,
            },
        },
    ])
}

pub(super) fn start_playback(
    generation: u64,
    path: PathBuf,
    size: TerminalSize,
    muted: bool,
    panel: PanelId,
    region: Region,
    media: &mut MediaRuntime,
) -> Result<Vec<Action>, String> {
    Ok(
        match media.start(generation, &path, size, muted, panel, region) {
            Ok(action) => vec![action],
            Err(message) => vec![Action::PreviewPlaybackFailed {
                generation,
                path,
                message,
            }],
        },
    )
}
