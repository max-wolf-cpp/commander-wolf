use std::fs;
use std::io;
use std::path::Path;

use crate::file_operations::FileOperationOutcome;
use crate::file_operations::progress::OperationObserver;

use super::copy::copy_path_observed;
use super::removal::{remove_path, remove_source_observed};
use super::staging::{rename_checked, unique_staging_path};

pub(super) fn move_path_observed(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    observer.begin_file(source, 1);
    observer.phase("Moving");
    if !overwrite
        || matches!(fs::symlink_metadata(destination), Err(error) if error.kind() == io::ErrorKind::NotFound)
    {
        return move_path_with(
            source,
            destination,
            |from, to| rename_checked(from, to, "move"),
            remove_source_observed,
            observer,
        );
    }

    move_with_overwrite(source, destination, observer)
}

fn move_with_overwrite(
    source: &Path,
    destination: &Path,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    let Some(parent) = destination.parent() else {
        return FileOperationOutcome::failure(format!(
            "Destination '{}' has no parent",
            destination.display()
        ));
    };

    let backup = match unique_staging_path(parent) {
        Ok(backup) => backup,
        Err(error) => return FileOperationOutcome::failure(error),
    };
    if let Err(error) = rename_checked(destination, &backup, "backup") {
        return FileOperationOutcome::failure(format!(
            "Unable to preserve existing destination '{}': {error}",
            destination.display()
        ));
    }

    match rename_checked(source, destination, "move") {
        Ok(()) => {
            observer.finish_file();
            observer.phase("Finalizing");
            finish_overwritten_move(source, destination, &backup)
        }
        Err(error) if is_cross_device(&error) => {
            if let Err(restore) = rename_checked(&backup, destination, "restore") {
                return FileOperationOutcome::changed_failure(format!(
                    "Cannot prepare cross-device move from '{}' to '{}': {error}; restoring the previous destination from '{}' also failed: {restore}",
                    source.display(),
                    destination.display(),
                    backup.display()
                ));
            }

            copy_then_remove(source, destination, true, remove_source_observed, observer)
        }
        Err(error) => match rename_checked(&backup, destination, "restore") {
            Ok(()) => FileOperationOutcome::failure(format!(
                "Unable to move '{}' to '{}': {error}",
                source.display(),
                destination.display()
            )),
            Err(restore) => FileOperationOutcome::changed_failure(format!(
                "Unable to move '{}' to '{}': {error}; restoring the previous destination from '{}' also failed: {restore}",
                source.display(),
                destination.display(),
                backup.display()
            )),
        },
    }
}

pub(super) fn move_path_with<R, D>(
    source: &Path,
    destination: &Path,
    rename: R,
    remove: D,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome
where
    R: FnOnce(&Path, &Path) -> io::Result<()>,
    D: FnOnce(&Path, &mut dyn OperationObserver) -> io::Result<()>,
{
    match rename(source, destination) {
        Ok(()) => {
            observer.finish_file();
            FileOperationOutcome::success(format!(
                "Moved '{}' to '{}'",
                source.display(),
                destination.display()
            ))
        }
        Err(error) if is_cross_device(&error) => {
            copy_then_remove(source, destination, false, remove, observer)
        }
        Err(error) => FileOperationOutcome::failure(format!(
            "Unable to move '{}' to '{}': {error}",
            source.display(),
            destination.display()
        )),
    }
}

fn finish_overwritten_move(
    source: &Path,
    destination: &Path,
    backup: &Path,
) -> FileOperationOutcome {
    match remove_path(backup) {
        Ok(()) => FileOperationOutcome::success(format!(
            "Moved '{}' to '{}' with overwrite",
            source.display(),
            destination.display()
        )),
        Err(error) => FileOperationOutcome::changed_failure(format!(
            "Moved '{}' to '{}', but could not remove backup '{}': {error}",
            source.display(),
            destination.display(),
            backup.display()
        )),
    }
}

fn copy_then_remove<D>(
    source: &Path,
    destination: &Path,
    overwrite: bool,
    remove: D,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome
where
    D: FnOnce(&Path, &mut dyn OperationObserver) -> io::Result<()>,
{
    if let Err(error) = copy_path_observed(source, destination, overwrite, observer) {
        return error.into_outcome();
    }

    observer.phase("Removing source");
    match remove(source, observer) {
        Ok(()) => FileOperationOutcome::success(format!(
            "Moved '{}' to '{}' using cross-device copy{}",
            source.display(),
            destination.display(),
            if overwrite { " with overwrite" } else { "" }
        )),
        Err(error) => FileOperationOutcome::changed_failure(format!(
            "Copied '{}' to '{}', but could not remove the original: {error}",
            source.display(),
            destination.display()
        )),
    }
}

fn is_cross_device(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::CrossesDevices
}
