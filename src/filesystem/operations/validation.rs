use std::fs;
use std::io;
use std::path::Path;

use crate::file_operations::{FileOperation, FileOperationRequest};

pub(super) fn validate_operation(request: &FileOperationRequest) -> Result<(), String> {
    let operation = &request.operation;
    if let Some(source) = operation.source() {
        let metadata = fs::symlink_metadata(source)
            .map_err(|error| format!("Unable to inspect source '{}': {error}", source.display()))?;
        if metadata.file_type().is_dir() && !crate::filesystem::is_link(&metadata) {
            validate_directory(request, source)?;
        }
    }

    if let Some(destination) = destination(operation) {
        match fs::symlink_metadata(destination) {
            Ok(_) if !operation.overwrites() => {
                return Err(format!(
                    "Destination already exists: '{}'",
                    destination.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Unable to inspect destination '{}': {error}",
                    destination.display()
                ));
            }
        }
    }

    Ok(())
}

pub(super) fn destination(operation: &FileOperation) -> Option<&Path> {
    match operation {
        FileOperation::Copy { destination, .. }
        | FileOperation::Move { destination, .. }
        | FileOperation::Rename { destination, .. }
        | FileOperation::CreateDirectory { destination }
        | FileOperation::CreateFile { destination }
        | FileOperation::CreateArchive { destination, .. }
        | FileOperation::ExtractArchive { destination, .. } => Some(destination),
        FileOperation::Delete { .. } => None,
    }
}

fn validate_directory(request: &FileOperationRequest, source: &Path) -> Result<(), String> {
    let source = fs::canonicalize(source)
        .map_err(|error| format!("Unable to resolve source '{}': {error}", source.display()))?;
    let protected = fs::canonicalize(&request.protected_directory).map_err(|error| {
        format!(
            "Unable to resolve protected panel directory '{}': {error}",
            request.protected_directory.display()
        )
    })?;

    let operation = &request.operation;
    if matches!(
        operation,
        FileOperation::Move { .. } | FileOperation::Delete { .. } | FileOperation::Rename { .. }
    ) && protected.starts_with(&source)
    {
        return Err(format!(
            "Cannot {:?} directory '{}' because the other panel is open inside it",
            operation.kind(),
            source.display()
        ));
    }

    let (FileOperation::Copy { destination, .. } | FileOperation::Move { destination, .. }) =
        operation
    else {
        return Ok(());
    };

    let parent = destination
        .parent()
        .ok_or_else(|| format!("Destination '{}' has no parent", destination.display()))?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "Unable to resolve destination parent '{}': {error}",
            parent.display()
        )
    })?;
    if parent.starts_with(&source) {
        return Err(format!(
            "Cannot {:?} directory '{}' into its own subdirectory '{}'",
            operation.kind(),
            source.display(),
            destination.display()
        ));
    }

    Ok(())
}
