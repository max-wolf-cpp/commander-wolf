use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::file_operations::FileOperationOutcome;
use crate::file_operations::progress::OperationObserver;

mod create;
mod extract;
mod paths;

const MAX_ENTRIES: usize = 100_000;
const MAX_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024 * 1024;

pub(super) fn create(
    sources: &[PathBuf],
    destination: &Path,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    staged(destination, false, observer, |staging, observer| {
        create::write(sources, destination, staging, observer)
    })
}

pub(super) fn extract(
    source: &Path,
    destination: &Path,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    staged(destination, true, observer, |staging, observer| {
        extract::unpack(source, staging, observer)
    })
}

fn staged(
    destination: &Path,
    directory: bool,
    observer: &mut dyn OperationObserver,
    operation: impl FnOnce(&Path, &mut dyn OperationObserver) -> Result<(), String>,
) -> FileOperationOutcome {
    let prepared = || {
        observer.check_cancelled()?;
        let name = destination
            .file_name()
            .ok_or("Destination has no file name")?;
        let parent = destination.parent().ok_or("Destination has no parent")?;
        let parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
        let destination = parent.join(name);
        let staging = super::staging::unique_staging_path(&parent)?;
        Ok::<_, String>((destination, staging))
    };
    let (destination, staging) = match prepared() {
        Ok(paths) => paths,
        Err(error) => return FileOperationOutcome::failure(error),
    };

    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    if let Err(error) = builder.create(&staging) {
        return FileOperationOutcome::failure(error.to_string());
    }

    let output = if directory {
        staging.clone()
    } else {
        staging.join("archive.zip")
    };
    let result = (|| {
        operation(&output, observer)?;
        observer.check_cancelled()?;
        observer.phase("Publishing");
        super::staging::publish_staging(&output, &destination)
    })();

    match result {
        Ok(()) => {
            if !directory && let Err(error) = fs::remove_dir(&staging) {
                return FileOperationOutcome::changed_failure(format!(
                    "Created '{}', but cannot remove temporary directory '{}': {error}",
                    destination.display(),
                    staging.display()
                ));
            }

            FileOperationOutcome::success(format!(
                "{} '{}'",
                if directory {
                    "Extracted to"
                } else {
                    "Created archive"
                },
                destination.display()
            ))
        }
        Err(error) => cleanup_failed(
            staging,
            format!(
                "Cannot {} '{}': {error}",
                if directory {
                    "extract to"
                } else {
                    "create archive"
                },
                destination.display()
            ),
        ),
    }
}

fn cleanup_failed(staging: PathBuf, error: String) -> FileOperationOutcome {
    let cleanup = fs::remove_dir_all(&staging);
    match cleanup {
        Ok(()) => FileOperationOutcome::failure(error),
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => {
            FileOperationOutcome::failure(error)
        }
        Err(cleanup) => FileOperationOutcome::changed_failure(format!(
            "{error}; cannot remove temporary archive output '{}': {cleanup}",
            staging.display()
        )),
    }
}

fn copy_stream(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    size: u64,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    let mut buffer = vec![0; 256 * 1024];
    let mut copied = 0u64;
    loop {
        observer.check_cancelled()?;
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }

        copied = copied
            .checked_add(count as u64)
            .ok_or("Archive entry size overflow")?;
        if copied > size {
            return Err("Archive entry exceeded its declared size or source file changed".into());
        }

        writer
            .write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
        observer.advance(count as u64);
    }

    if copied != size {
        return Err("Archive entry was truncated or source file changed".into());
    }

    observer.finish_file();
    Ok(())
}

#[cfg(test)]
mod tests;
