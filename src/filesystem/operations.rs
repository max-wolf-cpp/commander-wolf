use std::fs::{self, OpenOptions};
use std::path::Path;

use crate::file_operations::progress::{NoopObserver, OperationObserver};
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationOutcome, FileOperationRequest,
    OperationStatus,
};

mod archive;
mod copy;
mod moving;
mod removal;
mod staging;
mod validation;

use copy::copy_path_observed;
use moving::move_path_observed;
use removal::delete_path_observed;
use staging::rename_checked;
use validation::validate_operation;

#[cfg(test)]
use validation::destination;

pub fn execute_operation(request: &FileOperationRequest) -> FileOperationOutcome {
    execute_operation_observed(request, &mut NoopObserver)
}

pub(super) fn execute_operation_observed(
    request: &FileOperationRequest,
    observer: &mut dyn OperationObserver,
) -> FileOperationOutcome {
    if let Err(error) = observer.check_cancelled() {
        return FileOperationOutcome::failure(error);
    }

    if let Err(error) = validate_operation(request) {
        return FileOperationOutcome::failure(error);
    }

    #[cfg(test)]
    if let Err(error) = super::faults::check(
        "validated",
        destination(&request.operation).unwrap_or(&request.protected_directory),
    ) {
        return FileOperationOutcome::failure(error.to_string());
    }

    match &request.operation {
        FileOperation::Copy {
            source,
            destination,
            overwrite,
        } => match copy_path_observed(source, destination, *overwrite, observer) {
            Ok(()) => FileOperationOutcome::success(format!(
                "Copied '{}' to '{}'",
                source.display(),
                destination.display()
            )),
            Err(error) => error.into_outcome(),
        },
        FileOperation::Move {
            source,
            destination,
            overwrite,
        } => move_path_observed(source, destination, *overwrite, observer),
        FileOperation::Delete { source } => delete_path_observed(source, observer),
        FileOperation::Rename {
            source,
            destination,
        } => rename_path(source, destination),
        FileOperation::CreateDirectory { destination } => create_directory(destination),
        FileOperation::CreateFile { destination } => create_file(destination),
        FileOperation::CreateArchive {
            sources,
            destination,
        } => archive::create(sources, destination, observer),
        FileOperation::ExtractArchive {
            source,
            destination,
        } => archive::extract(source, destination, observer),
    }
}

pub fn execute_batch(request: &FileOperationBatchRequest) -> FileOperationOutcome {
    if request.operations.is_empty() {
        return FileOperationOutcome::failure("File operation batch is empty");
    }

    let expected_kind = request.operations[0].kind();
    if request
        .operations
        .iter()
        .any(|operation| operation.kind() != expected_kind)
    {
        return FileOperationOutcome::failure(
            "File operation batch contains mixed operation kinds",
        );
    }

    if request.operations.len() == 1 {
        return execute_operation(&FileOperationRequest {
            operation: request.operations[0].clone(),
            protected_directory: request.protected_directory.clone(),
            refresh: request.refresh.clone(),
        });
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut changed = false;
    let mut errors = Vec::new();
    for operation in &request.operations {
        let outcome = execute_operation(&FileOperationRequest {
            operation: operation.clone(),
            protected_directory: request.protected_directory.clone(),
            refresh: request.refresh.clone(),
        });
        changed |= outcome.filesystem_changed;
        if outcome.status == OperationStatus::Success {
            succeeded += 1;
        } else {
            failed += 1;
            errors.push(outcome.message);
        }
    }

    if failed == 0 {
        return FileOperationOutcome::success(format!(
            "Applied {succeeded} {} operation(s)",
            batch_kind_name(expected_kind)
        ));
    }

    FileOperationOutcome {
        status: if succeeded > 0 || changed {
            OperationStatus::Partial
        } else {
            OperationStatus::Failure
        },
        filesystem_changed: changed,
        message: format!(
            "{} batch: {succeeded} succeeded, {failed} failed: {}",
            batch_kind_name(expected_kind),
            errors.join("; ")
        ),
    }
}

fn rename_path(source: &Path, destination: &Path) -> FileOperationOutcome {
    rename_checked(source, destination, "rename")
        .map(|()| {
            FileOperationOutcome::success(format!(
                "Renamed '{}' to '{}'",
                source.display(),
                destination.display()
            ))
        })
        .unwrap_or_else(|error| {
            FileOperationOutcome::failure(format!(
                "Unable to rename '{}' to '{}': {error}",
                source.display(),
                destination.display()
            ))
        })
}

fn create_directory(destination: &Path) -> FileOperationOutcome {
    fs::create_dir(destination)
        .map(|()| {
            FileOperationOutcome::success(format!("Created directory '{}'", destination.display()))
        })
        .unwrap_or_else(|error| {
            FileOperationOutcome::failure(format!(
                "Unable to create directory '{}': {error}",
                destination.display()
            ))
        })
}

fn create_file(destination: &Path) -> FileOperationOutcome {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map(|_| FileOperationOutcome::success(format!("Created file '{}'", destination.display())))
        .unwrap_or_else(|error| {
            FileOperationOutcome::failure(format!(
                "Unable to create file '{}': {error}",
                destination.display()
            ))
        })
}

fn batch_kind_name(kind: crate::file_operations::FileOperationKind) -> &'static str {
    use crate::file_operations::FileOperationKind;
    match kind {
        FileOperationKind::Copy => "Copy",
        FileOperationKind::Move => "Move",
        FileOperationKind::Delete => "Delete",
        FileOperationKind::Rename => "Rename",
        FileOperationKind::CreateDirectory => "Create directory",
        FileOperationKind::CreateFile => "Create file",
        FileOperationKind::CreateArchive => "Create archive",
        FileOperationKind::ExtractArchive => "Extract archive",
    }
}

#[cfg(test)]
mod tests;
