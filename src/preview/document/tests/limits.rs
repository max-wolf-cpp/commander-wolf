use super::*;

#[test]
fn detects_supported_document_extensions_case_insensitively() {
    for (path, format) in [
        ("book.PDF", DocumentFormat::Pdf),
        ("scan.djv", DocumentFormat::DjVu),
        ("comic.cbz", DocumentFormat::Cbz),
        ("book.epub", DocumentFormat::Epub),
        ("book.mobi", DocumentFormat::Mobi),
        ("book.azw3", DocumentFormat::Azw3),
        ("book.kfx", DocumentFormat::Kfx),
        ("book.fb2", DocumentFormat::Fb2),
    ] {
        assert_eq!(DocumentFormat::from_path(Path::new(path)), Some(format));
    }

    assert_eq!(DocumentFormat::from_path(Path::new("notes.md")), None);
}

#[test]
fn render_size_preserves_ratio_and_honors_pixel_limit() {
    let request = DocumentRequest {
        generation: 1,
        request_id: 1,
        path: "page.pdf".into(),
        format: DocumentFormat::Pdf,
        page: 0,
        pixel_width: 4000,
        pixel_height: 4000,
        text_columns: 80,
        text_rows: 20,
        options: DocumentOptions {
            max_render_pixels: 1_000_000,
            ..DocumentOptions::default()
        },
    };

    let (width, height) = fit_size(4000, 2000, &request);
    assert_eq!((width, height), (1414, 707));
    assert!(u64::from(width) * u64::from(height) <= 1_000_000);

    for (source_width, source_height) in [(100_000, 1), (1, 100_000), (1, 1), (u32::MAX, u32::MAX)]
    {
        for budget in [1, 2, 7, 100] {
            let mut request = request.clone();
            request.options.max_render_pixels = budget;
            let (width, height) = fit_size(source_width, source_height, &request);
            assert!(u64::from(width) * u64::from(height) <= budget);
        }
    }
}

#[test]
fn archive_limits_reject_before_unpacking_and_truncated_formats_fail() {
    let archive = small_cbz();
    assert!(
        check_archive(&archive, 1, u64::MAX)
            .unwrap_err()
            .contains("entry limit")
    );
    assert!(
        check_archive(&archive, 10, 1)
            .unwrap_err()
            .contains("byte limit")
    );

    let temp = crate::test_support::TempDirectory::new();
    for (extension, format) in [
        ("pdf", DocumentFormat::Pdf),
        ("djvu", DocumentFormat::DjVu),
        ("cbz", DocumentFormat::Cbz),
        ("epub", DocumentFormat::Epub),
        ("fb2", DocumentFormat::Fb2),
        ("mobi", DocumentFormat::Mobi),
        ("azw3", DocumentFormat::Azw3),
        ("kfx", DocumentFormat::Kfx),
    ] {
        let path = temp.join(format!("broken.{extension}"));
        fs::write(&path, b"truncated").unwrap();
        assert!(load(&request(&path, format)).is_err(), "{extension}");
    }
}
