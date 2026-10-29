use super::*;

#[test]
fn ebook_pagination_preserves_all_text_across_small_pages_and_resize() {
    let temp = crate::test_support::TempDirectory::new();
    let path = temp.join("book.fb2");
    let text = "a\nb\n\nc\nLongUnbrokenWord0123456789\nПривет мир\n中文測試\n🙂👩‍💻 e\u{301}";
    let paragraphs = text
        .split('\n')
        .map(|line| format!("<p>{line}</p>"))
        .collect::<String>();
    fs::write(&path, format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><description><title-info><book-title>Fixture</book-title><lang>en</lang></title-info></description><body><section>{paragraphs}</section></body></FictionBook>")).unwrap();

    // Whitespace is normalized by the ebook decoder and word wrapper. Every
    // non-whitespace scalar, including combining marks, must occur once.
    let normalize = |value: &str| {
        value
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    for (columns, rows) in [(1, 1), (2, 2), (5, 1), (8, 3), (40, 10)] {
        let mut request = request(&path, DocumentFormat::Fb2);
        request.text_columns = columns;
        request.text_rows = rows;
        if columns == 1 {
            assert!(load(&request).unwrap_err().contains("too narrow"));
            continue;
        }

        let first = load(&request).unwrap();
        let mut all = String::new();
        for page in 0..first.preview.total_pages {
            request.page = page;
            let output = load(&request).unwrap();
            assert_eq!(output.preview.current_page, page);
            let DocumentPageContent::Text(lines) = output.preview.content else {
                panic!("expected text")
            };
            assert!(lines.len() <= usize::from(rows));
            for line in lines {
                assert!(line.width() <= usize::from(columns));
                all.push_str(&line);
            }
        }

        assert_eq!(
            normalize(&all),
            normalize(text),
            "viewport {columns}x{rows}"
        );

        request.page = usize::MAX;
        assert_eq!(
            load(&request).unwrap().preview.current_page,
            first.preview.total_pages - 1
        );
    }
}

#[test]
fn empty_book_and_empty_sections_have_one_empty_page() {
    let temp = crate::test_support::TempDirectory::new();
    let path = temp.join("empty.fb2");
    // A valid FB2 body contains at least one section, even when it is empty.
    for body in [
        "<section/>",
        "<section><p></p></section><section><p> </p></section>",
    ] {
        fs::write(&path, format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><description><title-info><book-title>Empty</book-title><lang>en</lang></title-info></description><body>{body}</body></FictionBook>")).unwrap();
        let mut request = request(&path, DocumentFormat::Fb2);
        request.page = usize::MAX;
        request.text_columns = 1;
        request.text_rows = 1;

        let result = load(&request).unwrap();
        assert_eq!(result.preview.current_page, 0);
        assert_eq!(result.preview.total_pages, 1);
        assert!(
            matches!(result.preview.content, DocumentPageContent::Text(lines) if lines.is_empty())
        );
    }
}

#[test]
fn wrapping_preserves_empty_paragraphs_and_grapheme_boundaries() {
    assert!(wrap_text("", 0).is_empty());
    assert_eq!(wrap_text("a\n\nb", 1), ["a", "", "b"]);
    assert_eq!(wrap_text("e\u{301}e\u{301}", 1), ["e\u{301}", "e\u{301}"]);
    assert_eq!(wrap_text("中文🙂", 2), ["中", "文", "🙂"]);
    assert_eq!(wrap_text("abcdefgh", 3), ["abc", "def", "gh"]);
}

#[test]
fn self_authored_mobi_prc_and_kfx_fixtures_return_actual_text() {
    let temp = crate::test_support::TempDirectory::new();
    let text = "Wolf release fixture text.";
    let book = ebook_rs::TxtBook::parse(text.as_bytes(), "Wolf", false).unwrap();
    let kfx = ebook_rs::UniversalKfxExporter::export(&book).unwrap();
    for (extension, format, bytes) in [
        ("mobi", DocumentFormat::Mobi, mobi_fixture(false)),
        ("prc", DocumentFormat::Mobi, mobi_fixture(false)),
        ("kfx", DocumentFormat::Kfx, kfx),
    ] {
        let path = temp.join(format!("fixture.{extension}"));
        fs::write(&path, bytes).unwrap();
        let result = load(&request(&path, format)).unwrap();
        let DocumentPageContent::Text(lines) = result.preview.content else {
            panic!("expected text")
        };
        assert!(lines.join(" ").contains(text), "{extension}: {lines:?}");
    }

    let protected = temp.join("protected.mobi");
    fs::write(&protected, mobi_fixture(true)).unwrap();
    assert!(
        load(&request(&protected, DocumentFormat::Mobi))
            .unwrap_err()
            .contains("DRM")
    );
}

#[test]
fn epub_lazy_sections_are_loaded_and_protection_is_not_plain_text_fallback() {
    let temp = crate::test_support::TempDirectory::new();
    let container = b"<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"book.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>";
    let package = b"<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"2.0\" unique-identifier=\"id\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>Wolf</dc:title><dc:identifier id=\"id\">wolf-fixture</dc:identifier><dc:language>en</dc:language></metadata><manifest><item id=\"one\" href=\"one.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"one\"/></spine></package>";
    let chapter = b"<html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>Wolf</title></head><body><p>Wolf archive fixture.</p></body></html>";
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("book.opf", package),
        ("one.xhtml", chapter),
    ];

    let path = temp.join("book.epub");
    fs::write(&path, zip_entries(&entries)).unwrap();
    let result = load(&request(&path, DocumentFormat::Epub)).unwrap();
    assert!(
        matches!(result.preview.content, DocumentPageContent::Text(lines) if lines.join(" ").contains("Wolf archive fixture."))
    );

    entries.push(("META-INF/rights.xml", b"<rights/>"));
    fs::write(&path, zip_entries(&entries)).unwrap();
    assert!(
        load(&request(&path, DocumentFormat::Epub))
            .unwrap_err()
            .contains("DRM")
    );
}

fn mobi_fixture(encrypted: bool) -> Vec<u8> {
    // A self-authored uncompressed MOBI 6 / Palm database with one text record.
    let html = b"<html><body><p>Wolf release fixture text.</p></body></html>";
    let mut bytes = vec![0; 96 + 16 + 232];
    bytes[..4].copy_from_slice(b"Wolf");
    bytes[60..68].copy_from_slice(b"BOOKMOBI");
    bytes[76..78].copy_from_slice(&2u16.to_be_bytes());
    bytes[78..82].copy_from_slice(&96u32.to_be_bytes());
    let offset = bytes.len() as u32;
    bytes[86..90].copy_from_slice(&offset.to_be_bytes());

    let header = &mut bytes[96..];
    header[0..2].copy_from_slice(&1u16.to_be_bytes());
    header[4..8].copy_from_slice(&(html.len() as u32).to_be_bytes());
    header[8..10].copy_from_slice(&1u16.to_be_bytes());
    header[10..12].copy_from_slice(&4096u16.to_be_bytes());
    header[12..14].copy_from_slice(&u16::from(encrypted).to_be_bytes());
    header[16..20].copy_from_slice(b"MOBI");
    header[20..24].copy_from_slice(&232u32.to_be_bytes());
    header[24..28].copy_from_slice(&2u32.to_be_bytes());
    header[28..32].copy_from_slice(&65001u32.to_be_bytes());
    header[36..40].copy_from_slice(&6u32.to_be_bytes());

    bytes.extend_from_slice(html);

    bytes
}
