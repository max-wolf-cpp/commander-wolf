use super::file_operations::is_destructive;
use super::playback::stop_video;
use super::preview::prepare_selected;
use crate::app::{Action, AppState, Effect, Notification};
use crate::file_operations::{PanelRefreshRequest, RefreshContext, RefreshSelection};
use crate::workspace::PanelId;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::RefreshPanels => vec![Effect::RefreshPanels {
            requests: refresh_requests(state, None, None),
            context: RefreshContext::Manual,
        }],
        Action::PanelsRefreshed { results, context } => {
            apply_refresh_results(state, results, context)
        }
        Action::WatchedPanelsRefreshed {
            results,
            changed_paths,
        } => apply_watched_refresh(state, *results, &changed_paths),
        _ => unreachable!("dispatcher routed an unsupported action to refresh"),
    }
}

pub(super) fn refresh_requests(
    state: &AppState,
    active_selection: Option<RefreshSelection>,
    other_selection: Option<RefreshSelection>,
) -> [PanelRefreshRequest; 2] {
    [PanelId::Left, PanelId::Right].map(|panel| {
        let panel_state = state.workspace.panel(panel);
        let explicit = if panel == state.workspace.active_panel {
            active_selection.clone()
        } else {
            other_selection.clone()
        };
        PanelRefreshRequest {
            panel,
            directory: panel_state.directory.clone(),
            selection: explicit.unwrap_or_else(|| RefreshSelection {
                preferred: panel_state.selected_path().map(ToOwned::to_owned),
                fallback_index: panel_state.selected,
            }),
        }
    })
}

pub(super) fn apply_refresh_results(
    state: &mut AppState,
    results: [crate::file_operations::PanelRefreshResult; 2],
    context: RefreshContext,
) -> Vec<Effect> {
    state.search.clear_visuals_for(PanelId::Left);
    state.search.clear_visuals_for(PanelId::Right);

    let old_selected = state.selected_path().map(ToOwned::to_owned);
    let mut failures = Vec::new();
    let scroll_offset = state.workspace.scroll_offset;
    for result in results {
        match result.result {
            Ok(snapshot) => state
                .workspace
                .panel_mut(result.request.panel)
                .refresh_directory(snapshot, scroll_offset),
            Err(error) => failures.push((result.request.panel, error)),
        }
    }

    if failures.is_empty() {
        if matches!(context, RefreshContext::Manual) {
            state.notification = Some(Notification::info("Both panels refreshed"));
        }
    } else {
        let details = failures
            .into_iter()
            .map(|(panel, error)| format!("{} panel: {error}", panel_name(panel)))
            .collect::<Vec<_>>()
            .join("; ");
        state.notification = Some(Notification::error(match &context {
            RefreshContext::Manual => format!("Refresh failed for {details}"),
            RefreshContext::Operation { outcome, .. } => {
                format!(
                    "{}; filesystem changed, but refresh failed for {details}",
                    outcome.message
                )
            }
        }));
    }

    let selected_changed = old_selected.as_deref() != state.selected_path();
    let sync_preview = match context {
        RefreshContext::Manual => true,
        RefreshContext::Operation { kind, .. } => {
            is_destructive(kind) || selected_changed || state.preview.needs_surface_restore()
        }
    };
    if sync_preview {
        prepare_selected(state)
    } else {
        Vec::new()
    }
}

fn apply_watched_refresh(
    state: &mut AppState,
    results: [crate::file_operations::PanelRefreshResult; 2],
    changed_paths: &[std::path::PathBuf],
) -> Vec<Effect> {
    let previous_selection = state.selected_entry().cloned();
    let mut relocated = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        let panel = result.request.panel;
        let current = state.workspace.panel(panel);
        if current.directory != result.request.directory {
            continue;
        }

        match result.result {
            Ok(snapshot) => {
                if let Some((old, new)) = refresh_watched_snapshot(state, panel, snapshot) {
                    relocated.push((panel, old, new));
                }
            }
            Err(error) => failures.push((panel, error)),
        }
    }

    if !failures.is_empty() {
        let details = failures
            .into_iter()
            .map(|(panel, error)| format!("{} panel: {error}", panel_name(panel)))
            .collect::<Vec<_>>()
            .join("; ");
        state.notification = Some(Notification::error(format!(
            "Automatic refresh failed for {details}"
        )));
    } else if let Some((_, old, new)) = relocated.first() {
        state.notification = Some(Notification::info(format!(
            "'{}' is unavailable; opened '{}'",
            old.display(),
            new.display()
        )));
    }

    let selected = state.selected_entry();
    let selection_changed = previous_selection.as_ref() != selected;
    let selected_changed_on_disk = selected.is_some_and(|entry| {
        changed_paths.iter().any(|path| {
            path == &entry.path || entry.is_directory && path.parent() == Some(entry.path.as_path())
        })
    });
    if selection_changed || selected_changed_on_disk {
        let mut effects = stop_video(state);
        effects.extend(prepare_selected(state));
        effects
    } else {
        Vec::new()
    }
}

fn panel_name(panel: PanelId) -> &'static str {
    match panel {
        PanelId::Left => "left",
        PanelId::Right => "right",
    }
}

fn refresh_watched_snapshot(
    state: &mut AppState,
    panel: PanelId,
    mut snapshot: crate::workspace::DirectorySnapshot,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let current = state.workspace.panel(panel);
    if snapshot.directory == current.directory && snapshot.entries == current.all_entries {
        return None;
    }

    let preferred = current.selected_path();
    snapshot.selected = preferred
        .and_then(|path| snapshot.entries.iter().position(|entry| entry.path == path))
        .unwrap_or_else(|| {
            current
                .selected
                .min(snapshot.entries.len().saturating_sub(1))
        });
    let relocation = (snapshot.directory != current.directory)
        .then(|| (current.directory.clone(), snapshot.directory.clone()));

    state.search.clear_visuals_for(panel);
    let scroll_offset = state.workspace.scroll_offset;
    state
        .workspace
        .panel_mut(panel)
        .refresh_directory(snapshot, scroll_offset);

    relocation
}
