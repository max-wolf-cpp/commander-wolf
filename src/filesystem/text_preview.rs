use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::preview::TextPreview;

const MAX_PREVIEW_BYTES: u64 = 256 * 1024;
const MAX_PREVIEW_LINES: usize = 4096;
const MAX_PREVIEW_LINE_CHARS: usize = 4096;

pub fn load(path: &Path) -> Result<TextPreview, String> {
    load_with_highlighter(path, crate::syntax::default_highlighter())
}

pub fn load_with_highlighter(
    path: &Path,
    highlighter: &dyn crate::syntax::SyntaxHighlighter,
) -> Result<TextPreview, String> {
    let (bytes, byte_truncated) = read_prefix(path)?;
    let text = decoded_text(&bytes).ok_or_else(|| {
        format!(
            "No text preview for '{}': binary or unsupported encoding",
            path.display()
        )
    })?;

    let (lines, truncated) = preview_lines(&text, byte_truncated);
    let highlighting = highlighter.highlight(path, &lines);

    Ok(TextPreview {
        lines,
        highlighting,
        truncated,
    })
}

pub fn is_text(path: &Path) -> bool {
    read_prefix(path)
        .ok()
        .and_then(|(bytes, _)| decoded_text(&bytes))
        .is_some()
}

fn preview_lines(text: &str, mut truncated: bool) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    for line in text.lines() {
        if lines.len() == MAX_PREVIEW_LINES {
            truncated = true;
            break;
        }

        let expanded = line.replace('\t', "    ");
        if expanded.chars().count() > MAX_PREVIEW_LINE_CHARS {
            lines.push(expanded.chars().take(MAX_PREVIEW_LINE_CHARS).collect());
            truncated = true;
        } else {
            lines.push(expanded);
        }
    }

    (lines, truncated)
}

fn read_prefix(path: &Path) -> Result<(Vec<u8>, bool), String> {
    let file = File::open(path)
        .map_err(|error| format!("Cannot open text preview for '{}': {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read text preview for '{}': {error}", path.display()))?;

    let byte_truncated = bytes.len() as u64 > MAX_PREVIEW_BYTES;
    if byte_truncated {
        bytes.truncate(MAX_PREVIEW_BYTES as usize);
    }

    Ok((bytes, byte_truncated))
}

fn decoded_text(bytes: &[u8]) -> Option<String> {
    let text = decode_text(bytes)?;
    (!text.chars().any(is_binary_control)).then_some(text)
}

fn decode_text(bytes: &[u8]) -> Option<String> {
    if let Some(bytes) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return decode_utf8(bytes);
    }

    if let Some(bytes) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(bytes, u16::from_le_bytes);
    }

    if let Some(bytes) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(bytes, u16::from_be_bytes);
    }

    decode_utf8(bytes)
}

fn decode_utf8(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_owned()),
        Err(error)
            if error.error_len().is_none()
                && bytes.len().saturating_sub(error.valid_up_to()) < 4 =>
        {
            Some(
                std::str::from_utf8(&bytes[..error.valid_up_to()])
                    .ok()?
                    .to_owned(),
            )
        }
        Err(_) => None,
    }
}

fn decode_utf16(bytes: &[u8], convert: fn([u8; 2]) -> u16) -> Option<String> {
    let even_length = bytes.len() - bytes.len() % 2;
    let words = bytes[..even_length]
        .chunks_exact(2)
        .map(|chunk| convert([chunk[0], chunk[1]]));
    std::char::decode_utf16(words)
        .collect::<Result<String, _>>()
        .ok()
}

fn is_binary_control(character: char) -> bool {
    character.is_control() && !matches!(character, '\n' | '\r' | '\t')
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TempFile(PathBuf);

    impl TempFile {
        fn new(extension: &str, bytes: &[u8]) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "commander-wolf-text-preview-{}-{nonce}{extension}",
                std::process::id()
            ));
            fs::write(&path, bytes).unwrap();

            Self(path)
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn backend_failure_preserves_the_plain_preview() {
        struct FailedHighlighter;
        impl crate::syntax::SyntaxHighlighter for FailedHighlighter {
            fn highlight(
                &self,
                _: &Path,
                _: &[String],
            ) -> Result<Vec<Vec<crate::syntax::SyntaxSpan>>, String> {
                Err("test parser failure".into())
            }
        }

        let file = TempFile::new(".rs", b"let x = 42;");
        let preview = load_with_highlighter(&file.0, &FailedHighlighter).unwrap();

        assert_eq!(preview.lines, ["let x = 42;"]);
        assert_eq!(preview.highlighting.unwrap_err(), "test parser failure");
    }

    #[test]
    fn loads_unicode_source_markdown_json_and_extensionless_text() {
        for extension in [".rs", ".md", ".json", ""] {
            let file = TempFile::new(extension, "fn 狼() {\n\ttrue\n}\n".as_bytes());
            let preview = load(&file.0).unwrap();
            assert_eq!(preview.lines, ["fn 狼() {", "    true", "}"]);
            assert!(!preview.truncated);
        }
    }

    #[test]
    fn visual_studio_solution_is_text() {
        let file = TempFile::new(
            ".sln",
            b"Microsoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio Version 17\n",
        );
        assert!(is_text(&file.0));
        assert_eq!(load(&file.0).unwrap().lines.len(), 2);
    }

    #[test]
    fn supports_utf16_bom_and_rejects_binary_data() {
        let utf16 = TempFile::new(".txt", &[0xFF, 0xFE, b'W', 0, b'o', 0, b'l', 0, b'f', 0]);
        assert_eq!(load(&utf16.0).unwrap().lines, ["Wolf"]);

        let binary = TempFile::new(".bin", &[0, 1, 2, 3, 0xFF]);
        assert!(load(&binary.0).unwrap_err().contains("binary"));
        assert!(is_text(&utf16.0));
        assert!(!is_text(&binary.0));
    }

    #[test]
    fn bounds_large_text_without_rejecting_its_valid_prefix() {
        let file = TempFile::new(".log", &vec![b'a'; MAX_PREVIEW_BYTES as usize + 8]);
        let preview = load(&file.0).unwrap();
        assert!(preview.truncated);
        assert_eq!(preview.lines[0].len(), MAX_PREVIEW_LINE_CHARS);
    }
}
