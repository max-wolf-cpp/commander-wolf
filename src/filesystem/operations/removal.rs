use std::fs;
use std::io;
use std::path::Path;

use crate::file_operations::FileOperationOutcome;
use crate::file_operations::progress::OperationObserver;

pub(super) fn delete_path_observed(
    source: &Path,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) => {
            return FileOperationOutcome::failure(format!(
                "Unable to inspect '{}': {error}",
                source.display()
            ));
        }
    };

    observer.phase("Deleting");
    let result = remove_source_observed(source, observer);

    match result {
        Ok(()) => {
            FileOperationOutcome::success(format!("Deleted '{}' permanently", source.display()))
        }
        Err(error) if metadata.is_dir() => FileOperationOutcome::changed_failure(format!(
            "Unable to delete '{}' completely: {error}; the directory tree may be partially changed",
            source.display()
        )),
        Err(error) => FileOperationOutcome::failure(format!(
            "Unable to delete '{}': {error}",
            source.display()
        )),
    }
}

pub(super) fn remove_source_observed(
    path: &Path,
    observer: &mut dyn OperationObserver,
) -> io::Result<()> {
    observer
        .check_cancelled()
        .map_err(|message| io::Error::new(io::ErrorKind::Interrupted, message))?;
    #[cfg(test)]
    crate::filesystem::faults::check("remove_source", path)?;

    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !crate::filesystem::is_link(&metadata) {
        for item in fs::read_dir(path)? {
            remove_source_observed(&item?.path(), observer)?;
        }

        observer
            .check_cancelled()
            .map_err(|message| io::Error::new(io::ErrorKind::Interrupted, message))?;
        observer.begin_file(path, 1);
        fs::remove_dir(path)?;
    } else {
        observer.begin_file(path, 1);
        #[cfg(windows)]
        if crate::filesystem::is_directory_link(&metadata) {
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
        #[cfg(not(windows))]
        fs::remove_file(path)?;
    }

    observer.finish_file();

    Ok(())
}

pub(super) fn remove_path(path: &Path) -> io::Result<()> {
    #[cfg(test)]
    crate::filesystem::faults::check("cleanup", path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !crate::filesystem::is_link(&metadata) => {
            fs::remove_dir_all(path)
        }
        Ok(metadata) if crate::filesystem::is_directory_link(&metadata) => fs::remove_dir(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
