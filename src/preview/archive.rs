use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Tar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreviewEntry {
    pub path: String,
    pub is_directory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreview {
    pub entries: Vec<ArchivePreviewEntry>,
    pub truncated: bool,
}

impl ArchiveFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();

        match extension.as_str() {
            "zip" | "cbz" | "jar" | "war" | "ear" | "apk" => Some(Self::Zip),
            "7z" => Some(Self::SevenZip),
            "tar" => Some(Self::Tar),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_extensions_are_explicit_and_case_insensitive() {
        for path in ["wolf.zip", "book.CBZ", "app.jar", "bundle.7Z", "backup.tar"] {
            assert!(ArchiveFormat::from_path(Path::new(path)).is_some());
        }

        for path in ["notes.txt", "archive.rar", "bundle.tar.gz", "document.docx"] {
            assert!(ArchiveFormat::from_path(Path::new(path)).is_none());
        }
    }
}
