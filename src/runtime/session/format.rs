use std::path::{Path, PathBuf};

use crate::workspace::WorkspaceState;

// This is private state data, never executable Lua or user configuration.
#[cfg(windows)]
pub(super) const HEADER: &[u8; 8] = b"CWSESS1W";
#[cfg(not(windows))]
pub(super) const HEADER: &[u8; 8] = b"CWSESS1U";
pub(super) const MAX_BYTES: usize = 128 * 1024;

pub(super) fn encode(workspace: &WorkspaceState) -> Result<Vec<u8>, String> {
    let mut bytes = HEADER.to_vec();
    for panel in &workspace.panels {
        let directory = std::path::absolute(&panel.directory)
            .map_err(|error| format!("Cannot save session path: {error}"))?;
        let encoded = encode_path(&directory);
        if encoded.len() > MAX_BYTES || bytes.len() + 4 + encoded.len() > MAX_BYTES {
            return Err("Cannot save session: paths exceed the state size limit".into());
        }

        bytes.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&encoded);
    }

    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8]) -> Result<[PathBuf; 2], String> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(HEADER) {
        return Err("unsupported format or oversized state".into());
    }

    let mut remaining = &bytes[HEADER.len()..];
    let mut directories = [PathBuf::new(), PathBuf::new()];
    for directory in &mut directories {
        let length = remaining.get(..4).ok_or("missing path length")?;
        let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        remaining = &remaining[4..];
        let encoded = remaining.get(..length).ok_or("truncated path")?;
        *directory = decode_path(encoded)?;
        if !directory.is_absolute() {
            return Err("invalid or relative directory path".into());
        }

        remaining = &remaining[length..];
    }

    if !remaining.is_empty() {
        return Err("unexpected trailing data".into());
    }

    Ok(directories)
}

#[cfg(windows)]
pub(super) fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
pub(super) fn decode_path(bytes: &[u8]) -> Result<PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;

    if !bytes.len().is_multiple_of(2) {
        return Err("invalid Windows path encoding".into());
    }

    let words = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    if words.contains(&0) {
        return Err("null character in path".into());
    }

    Ok(std::ffi::OsString::from_wide(&words).into())
}

#[cfg(not(windows))]
pub(super) fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(windows))]
pub(super) fn decode_path(bytes: &[u8]) -> Result<PathBuf, String> {
    use std::os::unix::ffi::OsStringExt;

    if bytes.contains(&0) {
        return Err("null character in path".into());
    }

    Ok(std::ffi::OsString::from_vec(bytes.to_vec()).into())
}
