use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use super::paths::EntryPaths;
use crate::file_operations::progress::OperationObserver;

struct SourceEntry {
    source: PathBuf,
    name: String,
    directory: bool,
    size: u64,
    mode: u32,
}

pub(super) fn write(
    sources: &[PathBuf],
    destination: &Path,
    staging: &Path,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    let entries = collect(sources, destination, observer)?;
    let total = entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(entry.size.max(1))
            .ok_or("Archive source size overflow")
    })?;
    observer.set_total(total);
    observer.phase("Archiving");

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging)
        .map_err(|error| error.to_string())?;
    let mut archive = zip::ZipWriter::new(file);
    for entry in entries {
        observer.check_cancelled()?;
        observer.begin_file(&entry.source, entry.size);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(entry.mode)
            .large_file(entry.size >= u32::MAX as u64);
        if entry.directory {
            archive
                .add_directory(entry.name, options)
                .map_err(|error| error.to_string())?;
            observer.finish_file();
            continue;
        }

        let metadata = fs::symlink_metadata(&entry.source).map_err(|error| error.to_string())?;
        if crate::filesystem::is_link(&metadata) || !metadata.is_file() {
            return Err(format!(
                "Archive source changed or is a link: '{}'",
                entry.source.display()
            ));
        }

        let mut input = File::open(&entry.source).map_err(|error| error.to_string())?;
        archive
            .start_file(entry.name, options)
            .map_err(|error| error.to_string())?;
        super::copy_stream(&mut input, &mut archive, entry.size, observer)?;
    }

    let file = archive.finish().map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn collect(
    sources: &[PathBuf],
    destination: &Path,
    observer: &mut dyn OperationObserver,
) -> Result<Vec<SourceEntry>, String> {
    observer.phase("Scanning archive sources");
    if sources.is_empty() {
        return Err("No entries selected to archive".into());
    }

    let parent = fs::canonicalize(
        destination
            .parent()
            .ok_or("Archive destination has no parent")?,
    )
    .map_err(|error| error.to_string())?;
    let mut pending = Vec::new();
    for source in sources {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Archive source has no Unicode file name")?;
        if name.contains('\\') {
            return Err("Archive source names cannot contain backslashes".into());
        }

        pending.push((source.clone(), name.to_owned()));
        let metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
        if metadata.is_dir()
            && parent.starts_with(fs::canonicalize(source).map_err(|error| error.to_string())?)
        {
            return Err("Cannot create an archive inside a selected directory".into());
        }
    }

    let mut entries = Vec::new();
    let mut names = EntryPaths::default();
    while let Some((source, name)) = pending.pop() {
        observer.check_cancelled()?;
        let metadata = fs::symlink_metadata(&source).map_err(|error| error.to_string())?;
        observer.begin_file(&source, metadata.len());
        if crate::filesystem::is_link(&metadata) || !(metadata.is_dir() || metadata.is_file()) {
            return Err(format!(
                "Cannot archive links or special files: '{}'",
                source.display()
            ));
        }
        let path = names.register(&name, metadata.is_dir())?;
        // ZIP entry names always use forward slashes, independently of the host OS.
        let name = path
            .iter()
            .map(|part| part.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("/");
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o777
        };
        #[cfg(not(unix))]
        let mode = if metadata.is_dir() { 0o755 } else { 0o644 };
        entries.push(SourceEntry {
            source: source.clone(),
            name: name.clone(),
            directory: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            mode,
        });

        if metadata.is_dir() {
            for child in fs::read_dir(&source).map_err(|error| error.to_string())? {
                observer.check_cancelled()?;
                let child = child.map_err(|error| error.to_string())?;
                let child_name = child
                    .file_name()
                    .into_string()
                    .map_err(|_| "Archive source name is not Unicode")?;
                if child_name.contains('\\') {
                    return Err("Archive source names cannot contain backslashes".into());
                }
                if pending.len() + entries.len() >= super::MAX_ENTRIES {
                    return Err("Archive exceeds the 100,000 entry limit".into());
                }
                pending.push((child.path(), format!("{name}/{child_name}")));
            }
        }
    }

    Ok(entries)
}
