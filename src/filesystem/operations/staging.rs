use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn unique_staging_path(parent: &Path) -> Result<PathBuf, String> {
    for _ in 0..100 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".commander-wolf-stage-{}-{sequence}",
            std::process::id()
        ));
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(path),
            Ok(_) => {}
            Err(error) => {
                return Err(format!(
                    "Unable to inspect staging path '{}': {error}",
                    path.display()
                ));
            }
        }
    }

    Err(format!(
        "Unable to allocate staging path in '{}'",
        parent.display()
    ))
}

pub(super) fn publish_staging(staging: &Path, destination: &Path) -> Result<(), String> {
    rename_checked(staging, destination, "publish")
        .map_err(|error| format!("Unable to publish '{}': {error}", destination.display()))
}

pub(super) fn rename_checked(source: &Path, destination: &Path, _step: &str) -> io::Result<()> {
    #[cfg(test)]
    crate::filesystem::faults::check(_step, destination)?;
    crate::filesystem::rename::exclusive(source, destination)
}
