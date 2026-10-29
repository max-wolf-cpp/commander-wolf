use std::fs;
use std::io::Read;
use std::path::Path;

pub(super) const MAX_DOCUMENT_BYTES: u64 = 256 * 1024 * 1024;
pub(super) const MAX_DOCUMENT_PAGES: u64 = 50_000;

pub(super) fn ensure_size(path: &Path) -> Result<(), String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("Cannot inspect document: {error}"))?
        .len();
    if size > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "Document is too large for preview (limit: {} MiB)",
            MAX_DOCUMENT_BYTES / 1024 / 1024
        ));
    }

    Ok(())
}

pub(super) fn read_document(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;

    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("Document exceeds preview byte limit".into());
    }

    Ok(bytes)
}

pub(super) fn check_archive(
    bytes: &[u8],
    max_entries: usize,
    max_bytes: u64,
) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("Cannot open document archive: {error}"))?;
    if archive.len() > max_entries {
        return Err("Document archive exceeds entry limit".into());
    }

    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| error.to_string())?;
        total = total.saturating_add(entry.size());
        if total > max_bytes {
            return Err("Document archive exceeds unpacked byte limit".into());
        }

        if entry.encrypted() {
            return Err("Encrypted document archives are not supported".into());
        }
    }

    Ok(())
}
