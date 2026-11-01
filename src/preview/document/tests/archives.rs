use super::*;

#[test]
fn loads_epub_text_and_cbz_pages_without_extracting_to_disk() {
    let epub = temporary_path("epub");
    fs::write(&epub, ebook_rs::generate_sample_epub().unwrap()).unwrap();
    let output = load(&request(&epub, DocumentFormat::Epub)).unwrap();
    assert!(matches!(
        output.preview.content,
        DocumentPageContent::Text(_)
    ));
    assert!(output.preview.total_pages >= 1);

    let cbz = temporary_path("cbz");
    fs::write(&cbz, small_cbz()).unwrap();
    let output = load(&request(&cbz, DocumentFormat::Cbz)).unwrap();
    assert_eq!(output.preview.total_pages, 2);
    assert!(output.raster.is_some());

    let _ = fs::remove_file(epub);
    let _ = fs::remove_file(cbz);
}

#[test]
fn cbz_natural_order_pixel_content_source_limits_and_no_disk_extraction() {
    let temp = crate::test_support::TempDirectory::new();
    let mut pngs = Vec::new();
    for red in [10, 2, 1] {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([red, 0, 0, 255]),
        ))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
        pngs.push(bytes.into_inner());
    }

    let path = temp.join("ordered.cbz");
    fs::write(
        &path,
        zip_entries(&[
            ("page10.png", &pngs[0]),
            ("page2.png", &pngs[1]),
            ("page1.png", &pngs[2]),
            ("../outside.txt", b"never extract"),
        ]),
    )
    .unwrap();

    let mut request = request(&path, DocumentFormat::Cbz);
    for (page, red) in [1, 2, 10].into_iter().enumerate() {
        request.page = page;
        let result = load(&request).unwrap();
        assert_eq!(result.preview.total_pages, 3);
        assert!(
            result
                .raster
                .unwrap()
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [red, 0, 0, 255])
        );
    }
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);

    let mut bmp = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut bmp, image::ImageFormat::Bmp)
        .unwrap();
    let mut bmp = bmp.into_inner();
    bmp[18..22].copy_from_slice(&100_000i32.to_le_bytes());
    fs::write(&path, zip_entries(&[("huge.bmp", &bmp)])).unwrap();
    request.options.max_render_pixels = 1;
    assert!(load(&request).unwrap_err().to_lowercase().contains("limit"));
}
