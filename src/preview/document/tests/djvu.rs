use super::*;

#[test]
fn djvu_preview_limits_peak_work_without_rejecting_large_books_by_page_sum() {
    let limits = djvu_resource_limits(8_000_000);
    assert_eq!(limits.max_total_pixels, None);
    assert_eq!(limits.max_page_pixels, Some(256_000_000));
    assert_eq!(limits.max_decoded_bytes, Some(512 * 1024 * 1024));
    assert_eq!(limits.max_render_pixels, Some(8_000_000));
}

#[test]
fn loads_a_pure_rust_generated_multipage_djvu() {
    use djvu_rs::Bitmap;
    use djvu_rs::djvm::{DjvmSpool, DjvmStreamWriter};
    use djvu_rs::djvu_encode::{EncodeQuality, PageEncoder};

    let mut bitmap = Bitmap::new(8, 8);
    for index in 0..8 {
        bitmap.set(index, index, true);
    }
    let page = PageEncoder::from_bitmap(&bitmap)
        .with_quality(EncodeQuality::Lossless)
        .encode()
        .unwrap();

    let mut bundle = DjvmStreamWriter::new(Vec::new(), DjvmSpool::Memory).unwrap();
    bundle.add_component("page1.djvu", 1, &page).unwrap();
    bundle.add_component("page2.djvu", 1, &page).unwrap();
    let bytes = bundle.finish().unwrap();

    let path = temporary_path("djvu");
    fs::write(&path, bytes).unwrap();

    let mut second_page = request(&path, DocumentFormat::DjVu);
    second_page.page = 1;
    let output = load(&second_page).unwrap();

    assert_eq!(output.preview.current_page, 1);
    assert_eq!(output.preview.total_pages, 2);
    assert!(output.raster.is_some());
    let raster = output.raster.as_ref().unwrap();
    assert!(raster.pixels.chunks_exact(4).any(|pixel| pixel[0] < 80));
    assert!(raster.pixels.chunks_exact(4).any(|pixel| pixel[0] > 200));

    let _ = fs::remove_file(path);
}
