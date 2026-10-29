use std::sync::Arc;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings};

use super::input::read_document;
use super::raster::{RasterDocumentModel, fit_size, raster_output};
use super::{DocumentFormat, DocumentOutput, DocumentRequest, RasterFrame};

pub(super) fn load(request: &DocumentRequest) -> Result<DocumentOutput, String> {
    let bytes =
        read_document(&request.path).map_err(|error| format!("Cannot read PDF: {error}"))?;
    let pdf = Pdf::new(bytes).map_err(|error| match error {
        hayro::hayro_syntax::LoadPdfError::Decryption(_) => {
            "Encrypted PDF preview is not supported".to_owned()
        }
        hayro::hayro_syntax::LoadPdfError::Invalid => {
            "The PDF is damaged or unsupported".to_owned()
        }
    })?;

    let pages = pdf.pages();
    if pages.is_empty() {
        return Err("The PDF contains no pages".into());
    }

    let page_index = request.page.min(pages.len() - 1);
    let page = &pages[page_index];
    let (source_width, source_height) = page.render_dimensions();
    let (target_width, target_height) = fit_size(
        source_width.max(1.0) as u32,
        source_height.max(1.0) as u32,
        request,
    );

    let scale = (target_width as f32 / source_width.max(1.0))
        .min(target_height as f32 / source_height.max(1.0));
    let cache = RenderCache::new();
    let pixmap = hayro::render(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings {
            x_scale: scale,
            y_scale: scale,
            width: Some(target_width.min(u16::MAX as u32) as u16),
            height: Some(target_height.min(u16::MAX as u32) as u16),
            bg_color: WHITE,
        },
    );

    let width = u32::from(pixmap.width());
    let height = u32::from(pixmap.height());
    let raster = RasterFrame {
        width,
        height,
        stride: width as usize * 4,
        pixels: Arc::from(pixmap.data_as_u8_slice()),
    };

    Ok(raster_output(
        RasterDocumentModel {
            format: DocumentFormat::Pdf,
            current_page: page_index,
            total_pages: pages.len(),
            page_width: source_width.max(1.0) as u32,
            page_height: source_height.max(1.0) as u32,
            title: None,
            author: None,
        },
        raster,
    ))
}
