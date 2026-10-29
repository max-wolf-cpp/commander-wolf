use std::fs::File;
use std::path::Path;

use crate::preview::{ArchiveFormat, ArchivePreview, ArchivePreviewEntry};

const MAX_PREVIEW_ENTRIES: usize = 4096;
const MAX_PREVIEW_ENTRY_CHARS: usize = 4096;

pub fn load(path: &Path) -> Result<ArchivePreview, String> {
    let format = ArchiveFormat::from_path(path)
        .ok_or_else(|| format!("Unsupported archive format: '{}'", path.display()))?;
    let entries = match format {
        ArchiveFormat::Zip => load_zip(path),
        ArchiveFormat::SevenZip => load_seven_zip(path),
        ArchiveFormat::Tar => load_tar(path),
    }
    .map_err(|message| format!("Cannot preview archive '{}': {message}", path.display()))?;

    Ok(finish(entries))
}

fn load_zip(path: &Path) -> Result<Vec<ArchivePreviewEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut entries = Vec::with_capacity(archive.len().min(MAX_PREVIEW_ENTRIES + 1));
    for index in 0..archive.len().min(MAX_PREVIEW_ENTRIES + 1) {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| error.to_string())?;
        entries.push(ArchivePreviewEntry {
            path: normalize_entry_path(entry.name(), entry.is_dir()),
            is_directory: entry.is_dir(),
        });
    }

    Ok(entries)
}

fn load_seven_zip(path: &Path) -> Result<Vec<ArchivePreviewEntry>, String> {
    let archive = sevenz_rust2::Archive::open(path).map_err(|error| error.to_string())?;

    Ok(archive
        .files
        .iter()
        .take(MAX_PREVIEW_ENTRIES + 1)
        .map(|entry| ArchivePreviewEntry {
            path: normalize_entry_path(entry.name(), entry.is_directory()),
            is_directory: entry.is_directory(),
        })
        .collect())
}

fn load_tar(path: &Path) -> Result<Vec<ArchivePreviewEntry>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = tar::Archive::new(file);
    let entries = archive
        .entries_with_seek()
        .map_err(|error| error.to_string())?;

    entries
        .take(MAX_PREVIEW_ENTRIES + 1)
        .map(|entry| {
            let entry = entry.map_err(|error| error.to_string())?;
            let is_directory = entry.header().entry_type().is_dir();
            let path = entry.path().map_err(|error| error.to_string())?;
            Ok(ArchivePreviewEntry {
                path: normalize_entry_path(&path.to_string_lossy(), is_directory),
                is_directory,
            })
        })
        .collect()
}

fn finish(mut entries: Vec<ArchivePreviewEntry>) -> ArchivePreview {
    let truncated = entries.len() > MAX_PREVIEW_ENTRIES;
    entries.truncate(MAX_PREVIEW_ENTRIES);

    entries.sort_by(|left, right| {
        right.is_directory.cmp(&left.is_directory).then_with(|| {
            left.path
                .to_lowercase()
                .cmp(&right.path.to_lowercase())
                .then_with(|| left.path.cmp(&right.path))
        })
    });

    ArchivePreview { entries, truncated }
}

fn normalize_entry_path(path: &str, is_directory: bool) -> String {
    let mut characters = path.chars();
    let mut normalized = characters
        .by_ref()
        .take(MAX_PREVIEW_ENTRY_CHARS)
        .map(|character| {
            if character == '\\' {
                '/'
            } else if character.is_control() {
                '�'
            } else {
                character
            }
        })
        .collect::<String>();

    if characters.next().is_some() {
        normalized.push('…');
    }

    if is_directory {
        normalized.truncate(normalized.trim_end_matches('/').len());
    }

    if normalized.is_empty() {
        "(unnamed entry)".into()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TempArchive(PathBuf);

    impl TempArchive {
        fn path(extension: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "commander-wolf-archive-preview-{}-{nonce}{extension}",
                std::process::id()
            )))
        }
    }

    impl Drop for TempArchive {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn lists_zip_and_tar_entries_without_extracting_them() {
        let zip_path = TempArchive::path(".zip");
        let zip_file = File::create(&zip_path.0).unwrap();
        let mut zip = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.add_directory("Source/", options).unwrap();
        zip.start_file("README.md", options).unwrap();
        std::io::Write::write_all(&mut zip, b"Commander Wolf").unwrap();
        zip.finish().unwrap();

        let preview = load(&zip_path.0).unwrap();
        assert_eq!(preview.entries[0].path, "Source");
        assert!(preview.entries[0].is_directory);
        assert_eq!(preview.entries[1].path, "README.md");

        let tar_path = TempArchive::path(".tar");
        let tar_file = File::create(&tar_path.0).unwrap();
        let mut tar = tar::Builder::new(tar_file);
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_entry_type(tar::EntryType::Directory);
        header.set_cksum();
        tar.append_data(&mut header, "Assets/", std::io::empty())
            .unwrap();
        tar.finish().unwrap();

        assert_eq!(load(&tar_path.0).unwrap().entries[0].path, "Assets");
        assert_eq!(normalize_entry_path("bad\nname", false), "bad�name");
    }
}
