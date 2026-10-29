use std::sync::Arc;

use ebook_rs::cbz::CbzBook;
use image::ImageReader;

use super::input::{MAX_DOCUMENT_BYTES, MAX_DOCUMENT_PAGES, check_archive, read_document};
use super::raster::{RasterDocumentModel, fit_size, raster_output};
use super::{DocumentFormat, DocumentOutput, DocumentRequest, RasterFrame, non_empty};

pub(super) fn load(request: &DocumentRequest) -> Result<DocumentOutput, String> {
    let bytes =
        read_document(&request.path).map_err(|error| format!("Cannot read CBZ: {error}"))?;
    check_archive(&bytes, 50_000, 128 * 1024 * 1024)?;

    let fallback = request
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Comic Book");
    let book = CbzBook::parse(&bytes, fallback)
        .map_err(|error| format!("Cannot open CBZ document: {error}"))?;
    let total = book.sections.len();
    if total == 0 || total as u64 > MAX_DOCUMENT_PAGES {
        return Err("The CBZ has no pages or exceeds the page limit".into());
    }

    let page_index = request.page.min(total - 1);
    let (_, _, image_bytes) = CbzBook::prefetch_page_images(&book, page_index, 1)
        .into_iter()
        .next()
        .ok_or_else(|| "Cannot read the selected CBZ page".to_owned())?;

    let decoded = decode_page_image(image_bytes)?;
    let source_width = decoded.width();
    let source_height = decoded.height();
    let raster = thumbnail_page(&decoded, request);
    let metadata = book.metadata();

    Ok(raster_output(
        RasterDocumentModel {
            format: DocumentFormat::Cbz,
            current_page: page_index,
            total_pages: total,
            page_width: source_width,
            page_height: source_height,
            title: non_empty(&metadata.title),
            author: non_empty(metadata.creator()),
        },
        raster,
    ))
}

fn decode_page_image(image_bytes: Vec<u8>) -> Result<image::DynamicImage, String> {
    if image_bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("The selected CBZ page exceeds the decoded size limit".into());
    }

    let mut reader = ImageReader::new(std::io::Cursor::new(image_bytes))
        .with_guessed_format()
        .map_err(|error| format!("Cannot identify CBZ page image: {error}"))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|error| format!("Cannot decode CBZ page image: {error}"))?;
    let (source_width, source_height) = image::ImageDecoder::dimensions(&decoder);
    if u64::from(source_width) * u64::from(source_height) > 16_000_000 {
        return Err("CBZ source image exceeds decode pixel limit".into());
    }

    image::DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("Cannot decode CBZ page image: {error}"))
}

fn thumbnail_page(decoded: &image::DynamicImage, request: &DocumentRequest) -> RasterFrame {
    let source_width = decoded.width();
    let source_height = decoded.height();
    let (width, height) = fit_size(source_width, source_height, request);
    let rgba = decoded.thumbnail(width, height).into_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    RasterFrame {
        width,
        height,
        stride: width as usize * 4,
        pixels: Arc::from(rgba.into_raw()),
    }
}
