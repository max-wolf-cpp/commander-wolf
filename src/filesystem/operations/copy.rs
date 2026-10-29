use std::fs;
use std::path::Path;

use crate::file_operations::FileOperationOutcome;
use crate::file_operations::progress::OperationObserver;

use super::removal::remove_path;
use super::staging::{publish_staging, rename_checked, unique_staging_path};

pub(super) struct CopyFailure {
    message: String,
    changed: bool,
}

pub(super) fn copy_path_observed(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    observer: &mut dyn OperationObserver,
) -> Result<(), CopyFailure> {
    let parent = destination.parent().ok_or_else(|| CopyFailure {
        message: format!("Destination '{}' has no parent", destination.display()),
        changed: false,
    })?;
    let staging = unique_staging_path(parent).map_err(|message| CopyFailure {
        message,
        changed: false,
    })?;

    if let Err(error) = copy_to_staging(source, &staging, observer) {
        return Err(copy_failure(error, &staging, destination, false));
    }

    if let Err(error) = observer.check_cancelled() {
        return Err(copy_failure(error, &staging, destination, false));
    }

    // Publication/rollback is a transaction: defer cancellation until it is safe.
    observer.phase("Finalizing");
    if overwrite && fs::symlink_metadata(destination).is_ok() {
        return replace_with_staging(&staging, destination);
    }

    if let Err(error) = publish_staging(&staging, destination) {
        return Err(copy_failure(error, &staging, destination, false));
    }

    Ok(())
}

impl CopyFailure {
    pub(super) fn into_outcome(self) -> FileOperationOutcome {
        if self.changed {
            FileOperationOutcome::changed_failure(self.message)
        } else {
            FileOperationOutcome::failure(self.message)
        }
    }
}

fn replace_with_staging(staging: &Path, destination: &Path) -> Result<(), CopyFailure> {
    let parent = destination.parent().ok_or_else(|| CopyFailure {
        message: format!("Destination '{}' has no parent", destination.display()),
        changed: false,
    })?;
    let backup = unique_staging_path(parent)
        .map_err(|message| copy_failure(message, staging, destination, false))?;

    if let Err(error) = rename_checked(destination, &backup, "backup") {
        return Err(copy_failure(
            format!(
                "Unable to preserve existing destination '{}': {error}",
                destination.display()
            ),
            staging,
            destination,
            false,
        ));
    }

    if let Err(error) = publish_staging(staging, destination) {
        let restore = rename_checked(&backup, destination, "restore");
        let failure = copy_failure(error, staging, destination, false);
        let restore_failed = restore.is_err();
        return Err(CopyFailure {
            message: match restore {
                Ok(()) => failure.message,
                Err(restore) => format!(
                    "{}; restoring the previous destination from '{}' also failed: {restore}",
                    failure.message,
                    backup.display()
                ),
            },
            changed: failure.changed || restore_failed,
        });
    }

    remove_path(&backup).map_err(|error| CopyFailure {
        message: format!(
            "Overwrote '{}', but could not remove backup '{}': {error}",
            destination.display(),
            backup.display()
        ),
        changed: true,
    })?;

    Ok(())
}

fn copy_to_staging(
    source: &Path,
    staging: &Path,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    observer.check_cancelled()?;

    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Unable to inspect '{}': {error}", source.display()))?;
    if crate::filesystem::is_link(&metadata) {
        return Err(format!(
            "Cannot copy symlink '{}' safely; symlink copying is not supported",
            source.display()
        ));
    }

    if metadata.is_file() {
        observer.begin_file(source, metadata.len().max(1));
        let copy = crate::filesystem::copy_file::copy(source, staging, &metadata, observer);
        copy.map_err(|error| {
            format!(
                "Unable to copy '{}' to staging '{}': {error}",
                source.display(),
                staging.display()
            )
        })?;
        return Ok(());
    }

    if !metadata.is_dir() {
        return Err(format!(
            "Unsupported filesystem entry: '{}'",
            source.display()
        ));
    }

    copy_directory_to_staging(source, staging, &metadata, observer)
}

fn copy_directory_to_staging(
    source: &Path,
    staging: &Path,
    metadata: &fs::Metadata,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    fs::create_dir(staging).map_err(|error| {
        format!(
            "Unable to create staging directory '{}': {error}",
            staging.display()
        )
    })?;

    for item in fs::read_dir(source)
        .map_err(|error| format!("Unable to read directory '{}': {error}", source.display()))?
    {
        let item = item.map_err(|error| {
            format!(
                "Unable to enumerate directory '{}': {error}",
                source.display()
            )
        })?;
        copy_to_staging(&item.path(), &staging.join(item.file_name()), observer)?;
    }

    fs::set_permissions(staging, metadata.permissions()).map_err(|error| {
        format!(
            "Unable to preserve permissions for '{}': {error}",
            source.display()
        )
    })?;
    observer.begin_file(source, 1);
    observer.finish_file();

    Ok(())
}

fn copy_failure(
    error: String,
    staging: &Path,
    destination: &Path,
    destination_counts_as_change: bool,
) -> CopyFailure {
    let cleanup = remove_path(staging);
    let message = match cleanup {
        Ok(()) => error,
        Err(cleanup) => format!(
            "{error}; staging cleanup '{}' also failed: {cleanup}",
            staging.display()
        ),
    };

    CopyFailure {
        message,
        changed: fs::symlink_metadata(staging).is_ok()
            || destination_counts_as_change && fs::symlink_metadata(destination).is_ok(),
    }
}
