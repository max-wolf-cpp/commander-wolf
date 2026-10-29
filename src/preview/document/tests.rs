use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use unicode_width::UnicodeWidthStr;

use super::book::wrap_text;
use super::djvu::djvu_resource_limits;
use super::input::check_archive;
use super::raster::fit_size;
use super::*;

mod archives;
mod books;
mod djvu;
mod limits;
mod pdf;

fn request(path: &Path, format: DocumentFormat) -> DocumentRequest {
    DocumentRequest {
        generation: 1,
        request_id: 1,
        path: path.to_owned(),
        format,
        page: 0,
        pixel_width: 80,
        pixel_height: 60,
        text_columns: 40,
        text_rows: 10,
        options: DocumentOptions::default(),
    }
}

fn temporary_path(extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("commander-wolf-document-{unique}.{extension}"))
}

fn small_cbz() -> Vec<u8> {
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([20, 40, 60, 255]),
    ))
    .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
    .unwrap();

    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default();
        for name in ["page10.png", "page2.png"] {
            zip.start_file(name, options).unwrap();
            zip.write_all(&png).unwrap();
        }
        zip.finish().unwrap();
    }

    bytes
}

fn zip_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }

    zip.finish().unwrap().into_inner()
}
