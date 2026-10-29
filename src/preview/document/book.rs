use ebook_rs::Book;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::input::{check_archive, read_document};
use super::{
    DocumentFormat, DocumentOutput, DocumentPageContent, DocumentPreview, DocumentRequest,
    non_empty,
};

pub(super) fn load(request: &DocumentRequest) -> Result<DocumentOutput, String> {
    let bytes = read_document(&request.path)?;
    if request.format == DocumentFormat::Epub {
        check_archive(&bytes, 50_000, 128 * 1024 * 1024)?;
    }

    let book = decode_book(&bytes, request.format)?;
    let text = book_text(&book)?;
    let wrapped = wrap_text(&text, request.text_columns);
    if wrapped
        .iter()
        .any(|line| line.width() > usize::from(request.text_columns.max(1)))
    {
        return Err(
            "Preview is too narrow to display a complete text character; widen the preview".into(),
        );
    }

    let rows = usize::from(request.text_rows.max(1));
    let total = wrapped.len().div_ceil(rows).max(1);
    let page_index = request.page.min(total - 1);
    let lines = wrapped
        .into_iter()
        .skip(page_index * rows)
        .take(rows)
        .collect();
    let metadata = book.metadata();

    Ok(DocumentOutput {
        preview: DocumentPreview {
            format: request.format,
            current_page: page_index,
            total_pages: total,
            page_width: None,
            page_height: None,
            title: non_empty(&metadata.title),
            author: non_empty(metadata.creator()),
            content: DocumentPageContent::Text(lines),
        },
        raster: None,
    })
}

pub(super) fn wrap_text(text: &str, columns: u16) -> Vec<String> {
    let width = usize::from(columns.max(1));
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if !line.is_empty() && line.width() + 1 + word.width() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            for grapheme in word.graphemes(true) {
                // A wide grapheme cannot fit in a one-cell viewport. Keep it intact
                // so a later resize can reveal it; never split or discard its bytes.
                if !line.is_empty() && line.width() + grapheme.width() > width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push_str(grapheme);
            }
        }

        if !line.is_empty() {
            lines.push(line);
        }
    }

    lines
}

fn decode_book(bytes: &[u8], format: DocumentFormat) -> Result<Book, String> {
    // The generic book loader accepts arbitrary UTF-8 as plain text. Select the
    // declared decoder so truncated or protected books cannot silently succeed.
    let decoded = match format {
        DocumentFormat::Fb2 => ebook_rs::fb2::Fb2Book::parse(bytes),
        DocumentFormat::Mobi | DocumentFormat::Azw3 => ebook_rs::mobi::MobiBook::parse(bytes),
        DocumentFormat::Kfx => ebook_rs::kfx::KfxBook::parse(bytes),
        DocumentFormat::Epub => ebook_rs::archive::EpubArchive::from_bytes(bytes)
            .and_then(|archive| Book::from_archive_lazy(archive, "eBook")),
        _ => unreachable!("load_book only handles ebook formats"),
    };

    decoded.map_err(|error| {
        let text = error.to_string();
        if text.to_ascii_lowercase().contains("drm")
            || text.to_ascii_lowercase().contains("encrypted")
        {
            "This ebook is DRM-protected and cannot be previewed".to_owned()
        } else {
            format!("Cannot open ebook: {text}")
        }
    })
}

fn book_text(book: &Book) -> Result<String, String> {
    let sections = (0..book.sections.len())
        .map(|index| book.get_section(index))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sections
        .iter()
        .map(|section| section.plain_text.trim())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n"))
}
