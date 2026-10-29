use super::{
    DocumentFormat, DocumentOutput, DocumentPageContent, DocumentPreview, DocumentRequest,
    RasterFrame,
};

pub(super) struct RasterDocumentModel {
    pub(super) format: DocumentFormat,
    pub(super) current_page: usize,
    pub(super) total_pages: usize,
    pub(super) page_width: u32,
    pub(super) page_height: u32,
    pub(super) title: Option<String>,
    pub(super) author: Option<String>,
}

pub(super) fn raster_output(model: RasterDocumentModel, raster: RasterFrame) -> DocumentOutput {
    DocumentOutput {
        preview: DocumentPreview {
            format: model.format,
            current_page: model.current_page,
            total_pages: model.total_pages,
            page_width: Some(model.page_width),
            page_height: Some(model.page_height),
            title: model.title,
            author: model.author,
            content: DocumentPageContent::Raster {
                width: raster.width,
                height: raster.height,
                surface_ready: false,
            },
        },
        raster: Some(raster),
    }
}

pub(super) fn fit_size(
    source_width: u32,
    source_height: u32,
    request: &DocumentRequest,
) -> (u32, u32) {
    let max_width = request.pixel_width.max(1);
    let max_height = request.pixel_height.max(1);
    let scale = (max_width as f64 / source_width.max(1) as f64)
        .min(max_height as f64 / source_height.max(1) as f64)
        .min(1.0);
    let mut width = (source_width as f64 * scale).round().max(1.0) as u32;
    let mut height = (source_height as f64 * scale).round().max(1.0) as u32;

    let pixels = u64::from(width) * u64::from(height);
    if pixels > request.options.max_render_pixels {
        let limit_scale = (request.options.max_render_pixels as f64 / pixels as f64).sqrt();
        width = (width as f64 * limit_scale).floor().max(1.0) as u32;
        height = (height as f64 * limit_scale).floor().max(1.0) as u32;
        // Clamping the short edge to one pixel can violate the area budget for
        // very thin pages. Apply the remaining budget to the longer edge.
        if u64::from(width) * u64::from(height) > request.options.max_render_pixels {
            if width >= height {
                width = (request.options.max_render_pixels / u64::from(height)).max(1) as u32;
            } else {
                height = (request.options.max_render_pixels / u64::from(width)).max(1) as u32;
            }
        }
    }

    (width.min(u16::MAX as u32), height.min(u16::MAX as u32))
}
