use std::sync::Arc;

use djvu_rs::djvu_document::DjVuDocument;
use djvu_rs::djvu_render::{RenderOptions as DjVuRenderOptions, render_pixmap};
use djvu_rs::resource_limits::{ParseOptions, ResourceLimits};

use super::input::{MAX_DOCUMENT_BYTES, MAX_DOCUMENT_PAGES, read_document};
use super::raster::{RasterDocumentModel, fit_size, raster_output};
use super::{DocumentFormat, DocumentOutput, DocumentRequest, RasterFrame};

pub(super) fn load(request: &DocumentRequest) -> Result<DocumentOutput, String> {
    let bytes =
        read_document(&request.path).map_err(|error| format!("Cannot read DjVu: {error}"))?;
    let limits = djvu_resource_limits(request.options.max_render_pixels);
    let document = DjVuDocument::parse_with_options(
        &bytes,
        &ParseOptions {
            limits: Some(limits),
        },
    )
    .map_err(|error| format!("Cannot open DjVu document: {error}"))?;

    let total = document.page_count();
    if total == 0 {
        return Err("The DjVu document contains no pages".into());
    }

    let page_index = request.page.min(total - 1);
    let page = document
        .page(page_index)
        .map_err(|error| format!("Cannot read DjVu page: {error}"))?;
    let source_width = u32::from(page.width());
    let source_height = u32::from(page.height());
    let (target_width, target_height) = fit_size(source_width, source_height, request);
    let options = DjVuRenderOptions::fit_to_box(page, target_width, target_height);
    let pixmap = render_pixmap(page, &options)
        .map_err(|error| format!("Cannot render DjVu page: {error}"))?;

    let raster = RasterFrame {
        width: pixmap.width,
        height: pixmap.height,
        stride: pixmap.width as usize * 4,
        pixels: Arc::from(pixmap.data),
    };

    Ok(raster_output(
        RasterDocumentModel {
            format: DocumentFormat::DjVu,
            current_page: page_index,
            total_pages: total,
            page_width: source_width,
            page_height: source_height,
            title: None,
            author: None,
        },
        raster,
    ))
}

pub(super) fn djvu_resource_limits(max_render_pixels: u64) -> ResourceLimits {
    ResourceLimits {
        max_file_bytes: Some(MAX_DOCUMENT_BYTES),
        max_pages: Some(MAX_DOCUMENT_PAGES),
        max_components: Some(100_000),
        max_page_pixels: Some(256_000_000),
        // Preview touches one page at a time. Aggregate source pixels are not a
        // peak allocation and routinely exceed two billion in ordinary scans.
        max_total_pixels: None,
        max_decoded_bytes: Some(512 * 1024 * 1024),
        max_render_pixels: Some(max_render_pixels),
    }
}
