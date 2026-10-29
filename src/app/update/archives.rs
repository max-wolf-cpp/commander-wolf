use std::path::{Path, PathBuf};

use super::file_operations::{begin_batch_operation, begin_operation};
use super::file_requests::{selected_for_operation, targets_for_operation};
use super::refresh::refresh_requests;
use crate::app::{Action, AppState, Effect, Notification};
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationKind, FileOperationRequest,
    normalize_from,
};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    let result = match action {
        Action::CreateArchive(destination) => create(state, destination),
        Action::ExtractArchive {
            destination,
            other_panel,
        } => extract(state, destination, other_panel),
        _ => unreachable!("dispatcher routed an unsupported action to archives"),
    };

    result.unwrap_or_else(|error| {
        state.notification = Some(Notification::error(error));
        Vec::new()
    })
}

fn create(state: &mut AppState, destination: Option<String>) -> Result<Vec<Effect>, String> {
    let targets = targets_for_operation(state, FileOperationKind::CreateArchive)?;
    let Some(destination) = destination else {
        let name = if targets.len() == 1 {
            format!("{}.zip", targets[0].name)
        } else {
            "archive.zip".to_owned()
        };
        let effects = super::command_line::update(state, Action::EnterCommandMode);
        state.command_line = format!(
            ":archive \"{}\"",
            name.replace('\\', "\\\\").replace('"', "\\\"")
        );
        return Ok(effects);
    };

    if destination.trim().is_empty() {
        return Err("Usage: :archive [destination.zip]".into());
    }

    let mut destination =
        normalize_from(&state.workspace.active().directory, Path::new(&destination));
    if destination.extension().is_none() {
        destination.set_extension("zip");
    }
    if !destination
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err("Archive creation supports ZIP only; use a .zip destination".into());
    }

    let request = FileOperationBatchRequest {
        operations: vec![FileOperation::CreateArchive {
            sources: targets.into_iter().map(|entry| entry.path).collect(),
            destination: destination.clone(),
        }],
        source_panel: state.workspace.active_panel,
        protected_directory: state
            .workspace
            .panel(state.workspace.active_panel.other())
            .directory
            .clone(),
        refresh: refresh_for_destination(state, &destination),
    };
    state.notification = None;
    Ok(begin_batch_operation(state, request))
}

fn extract(
    state: &mut AppState,
    destination: Option<String>,
    other_panel: bool,
) -> Result<Vec<Effect>, String> {
    let selected = selected_for_operation(state, FileOperationKind::ExtractArchive)?;
    if selected.is_directory || crate::preview::ArchiveFormat::from_path(&selected.path).is_none() {
        return Err("Select a ZIP, TAR or 7z archive to extract".into());
    }

    let panel = if other_panel {
        state.workspace.active_panel.other()
    } else {
        state.workspace.active_panel
    };
    let destination = match destination {
        Some(path) if !path.trim().is_empty() => {
            normalize_from(&state.workspace.panel(panel).directory, Path::new(&path))
        }
        Some(_) => return Err("Usage: :extract [new-folder]".into()),
        None => {
            let name = selected
                .path
                .file_stem()
                .ok_or("Archive has no file name")?;
            if name.is_empty() {
                return Err("Archive has no destination folder name".into());
            }
            state.workspace.panel(panel).directory.join(name)
        }
    };
    let request = FileOperationRequest {
        operation: FileOperation::ExtractArchive {
            source: selected.path,
            destination: destination.clone(),
        },
        protected_directory: state
            .workspace
            .panel(state.workspace.active_panel.other())
            .directory
            .clone(),
        refresh: refresh_for_destination(state, &destination),
    };
    state.notification = None;
    Ok(begin_operation(state, request))
}

fn refresh_for_destination(
    state: &AppState,
    destination: &Path,
) -> [crate::file_operations::PanelRefreshRequest; 2] {
    let mut requests = refresh_requests(state, None, None);
    for request in &mut requests {
        if destination.parent() == Some(request.directory.as_path()) {
            request.selection.preferred = Some(PathBuf::from(destination));
        }
    }
    requests
}
