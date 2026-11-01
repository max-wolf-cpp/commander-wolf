use super::*;

#[test]
fn loads_first_pdf_page_and_reports_corruption() {
    let path = temporary_path("pdf");

    fs::write(&path, minimal_pdf(2)).unwrap();

    let output = load(&request(&path, DocumentFormat::Pdf)).unwrap();
    assert_eq!(output.preview.current_page, 0);
    assert_eq!(output.preview.total_pages, 2);
    assert!(output.raster.is_some());
    let raster = output.raster.as_ref().unwrap();
    assert!(
        raster
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[0] > 200 && pixel[1] < 30 && pixel[2] < 30)
    );

    fs::write(&path, b"not a pdf").unwrap();
    assert!(load(&request(&path, DocumentFormat::Pdf)).is_err());

    let _ = fs::remove_file(path);
}

#[test]
fn encrypted_and_empty_pdf_are_explicit_errors_and_page_counts_clamp() {
    let temp = crate::test_support::TempDirectory::new();
    let path = temp.join("fixture.pdf");
    // Self-authored blank page encrypted with pypdf 6.10.0, RC4-40; no
    // external document content. The password is wolf-user-password.
    fs::write(&path, include_bytes!("../../encrypted_fixture.pdf")).unwrap();
    assert!(
        load(&request(&path, DocumentFormat::Pdf))
            .unwrap_err()
            .contains("Encrypted PDF")
    );

    fs::write(&path, minimal_pdf(0)).unwrap();
    assert!(load(&request(&path, DocumentFormat::Pdf)).is_err());

    fs::write(&path, minimal_pdf(2)).unwrap();
    let mut request = request(&path, DocumentFormat::Pdf);
    request.page = usize::MAX;
    assert_eq!(load(&request).unwrap().preview.current_page, 1);

    request.options.max_render_pixels = 0;
    assert!(load(&request).unwrap_err().contains("positive"));
}

fn minimal_pdf(page_count: usize) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {page_count} >>",
            (0..page_count)
                .map(|index| format!("{} 0 R", index + 3))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    ];
    for _ in 0..page_count {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents {} 0 R >>",
            page_count + 3
        ));
    }

    let drawing = "1 0 0 rg 0 0 200 100 re f\n";
    objects.push(format!(
        "<< /Length {} >>\nstream\n{drawing}endstream",
        drawing.len()
    ));

    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }

    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.into_iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }

    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    bytes
}
