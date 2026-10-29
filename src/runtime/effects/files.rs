use std::path::PathBuf;

use crate::app::Action;
use crate::directory_edit::DirectoryEditRequest;
use crate::file_operations::{FileOperationBatchRequest, FileOperationRequest};
use crate::workspace::PanelId;

use super::super::operations::{OperationJob, OperationsRuntime};

#[cfg(test)]
mod tests;

pub(super) fn perform_operation(
    request: FileOperationRequest,
    operations: &mut OperationsRuntime,
) -> Result<Vec<Action>, String> {
    if matches!(
        request.operation.kind(),
        crate::file_operations::FileOperationKind::Copy
            | crate::file_operations::FileOperationKind::Move
            | crate::file_operations::FileOperationKind::Delete
            | crate::file_operations::FileOperationKind::CreateArchive
            | crate::file_operations::FileOperationKind::ExtractArchive
    ) {
        return Ok(operations.start(OperationJob::Single(request)));
    }

    let outcome = crate::filesystem::execute_operation(&request);
    Ok(vec![Action::FileOperationCompleted { request, outcome }])
}

pub(super) fn perform_batch(
    request: FileOperationBatchRequest,
    operations: &mut OperationsRuntime,
) -> Result<Vec<Action>, String> {
    if matches!(
        request.kind(),
        Some(
            crate::file_operations::FileOperationKind::Copy
                | crate::file_operations::FileOperationKind::Move
                | crate::file_operations::FileOperationKind::Delete
                | crate::file_operations::FileOperationKind::CreateArchive
                | crate::file_operations::FileOperationKind::ExtractArchive
        )
    ) {
        return Ok(operations.start(OperationJob::Batch(request)));
    }

    let outcome = crate::filesystem::execute_batch(&request);
    Ok(vec![Action::FileOperationBatchCompleted {
        request,
        outcome,
    }])
}

pub(super) fn apply_directory_edit(
    request: DirectoryEditRequest,
    operations: &mut OperationsRuntime,
) -> Result<Vec<Action>, String> {
    if request.operations.iter().any(|operation| {
        matches!(
            operation,
            crate::file_operations::FileOperation::Delete { .. }
        )
    }) {
        return Ok(operations.start(OperationJob::Edit(request)));
    }

    let (outcome, results) = crate::filesystem::edit::execute(&request);
    Ok(vec![Action::DirectoryEditCompleted {
        request,
        outcome,
        results: Box::new(results),
    }])
}

pub(super) fn reveal_tree_entry(
    generation: u64,
    panel: PanelId,
    path: PathBuf,
    is_directory: bool,
) -> Result<Vec<Action>, String> {
    let target = if is_directory {
        path.clone()
    } else {
        path.parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.clone())
    };
    let preferred = (!is_directory).then_some(path.as_path());

    crate::filesystem::load_snapshot(&target, preferred, 0)
        .map(|snapshot| {
            vec![Action::TreeRevealLoaded {
                generation,
                panel,
                directory: snapshot.directory,
                entries: snapshot.entries,
                selected: snapshot.selected,
            }]
        })
        .or_else(|message| {
            Ok(vec![Action::TreeLoadFailed {
                generation,
                message,
            }])
        })
}
