use std::fs;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::app::{Action, AppMode, AppState, Notification};
use crate::file_operations::{PanelRefreshRequest, PanelRefreshResult, RefreshSelection};
use crate::workspace::{DirectorySnapshot, PanelId};

use super::{DirectoryWatcher, RETRY_WATCH};

pub(super) struct RefreshResult {
    pub(super) results: [PanelRefreshResult; 2],
    pub(super) changed_paths: Vec<PathBuf>,
    pub(super) revisions: [u64; 2],
}

impl DirectoryWatcher {
    pub(super) fn collect_refresh(&mut self, now: Instant) -> Option<Action> {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            match self.worker.take().expect("finished worker exists").join() {
                Ok(result) => self.completed = Some(result),
                Err(_) => {
                    self.schedule(now);
                    return Some(Action::Notify(Notification::error(
                        "Directory refresh worker failed; retrying",
                    )));
                }
            }
        }

        None
    }

    pub(super) fn discard_stale_refresh(&mut self, state: &AppState, now: Instant) {
        if self
            .completed
            .as_ref()
            .is_some_and(|result| result.revisions != panel_revisions(state))
        {
            if let Some(result) = self.completed.take() {
                for path in result.changed_paths {
                    self.record_path(path, now);
                }
            }
            self.schedule(now);
        }
    }

    pub(super) fn apply_refresh(&mut self, state: &AppState) -> Option<Action> {
        let result = self.completed.take()?;
        let mut new_error = false;
        for item in &result.results {
            if item.request.directory != state.workspace.panel(item.request.panel).directory {
                continue;
            }

            let slot = &mut self.refresh_errors[item.request.panel.index()];
            match &item.result {
                Ok(_) => *slot = None,
                Err(error) => {
                    let marker = (item.request.directory.clone(), error.clone());
                    if slot.as_ref() != Some(&marker) {
                        new_error = true;
                        *slot = Some(marker);
                    }
                }
            }
        }

        if new_error || refresh_is_visible(&result, state) {
            return Some(Action::WatchedPanelsRefreshed {
                results: Box::new(result.results),
                changed_paths: result.changed_paths,
            });
        }

        None
    }

    pub(super) fn start_refresh(&mut self, state: &AppState, now: Instant) -> Option<Action> {
        if self.worker.is_some()
            || self.completed.is_some()
            || !self.due_at.is_some_and(|due| due <= now)
        {
            return None;
        }

        self.due_at = None;
        let directories = self.directories.clone();
        let revisions = panel_revisions(state);
        let changed_paths = if std::mem::take(&mut self.pending_all) {
            self.pending_paths.clear();
            state
                .workspace
                .preview_visible
                .then(|| state.selected_path().map(Path::to_path_buf))
                .flatten()
                .into_iter()
                .collect()
        } else {
            std::mem::take(&mut self.pending_paths)
                .into_iter()
                .collect()
        };

        match thread::Builder::new()
            .name("commander-wolf-directory-refresh".into())
            .spawn(move || refresh_directories(directories, changed_paths, revisions))
        {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => {
                self.pending_all = true;
                self.schedule(now + RETRY_WATCH);
                return Some(Action::Notify(Notification::error(format!(
                    "Cannot start directory refresh: {error}"
                ))));
            }
        }

        None
    }
}

pub(super) fn can_apply(state: &AppState) -> bool {
    state.mode == AppMode::Normal
        && state.operation_progress.is_none()
        && state.rename_prompt.is_none()
        && state.shell_output.is_none()
        && !state.viewer.is_open()
        && !state.directory_tree.open
        && !state.finder.is_open()
        && !state.directory_edit.active
}

pub(super) fn panel_revisions(state: &AppState) -> [u64; 2] {
    [PanelId::Left, PanelId::Right].map(|panel| state.workspace.panel(panel).revision)
}

pub(super) fn refresh_directories(
    directories: [PathBuf; 2],
    changed_paths: Vec<PathBuf>,
    revisions: [u64; 2],
) -> RefreshResult {
    let mut previous: Option<(PathBuf, Result<DirectorySnapshot, String>)> = None;
    let results = [PanelId::Left, PanelId::Right].map(|panel| {
        let directory = directories[panel.index()].clone();
        let result = if previous
            .as_ref()
            .is_some_and(|(path, _)| *path == directory)
        {
            previous
                .as_ref()
                .expect("matching previous snapshot")
                .1
                .clone()
        } else {
            let result = load_watched_snapshot(&directory);
            previous = Some((directory.clone(), result.clone()));
            result
        };
        PanelRefreshResult {
            request: PanelRefreshRequest {
                panel,
                directory,
                selection: RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            },
            result,
        }
    });

    RefreshResult {
        results,
        changed_paths,
        revisions,
    }
}

pub(super) fn load_watched_snapshot(directory: &Path) -> Result<DirectorySnapshot, String> {
    match fs::metadata(directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut parent = directory.parent();
            while let Some(path) = parent {
                if path.is_dir() {
                    return crate::filesystem::load_snapshot(path, Some(directory), 0);
                }

                parent = path.parent();
            }

            crate::filesystem::load_snapshot(directory, None, 0)
        }
        _ => crate::filesystem::load_snapshot(directory, None, 0),
    }
}

fn refresh_is_visible(result: &RefreshResult, state: &AppState) -> bool {
    result.results.iter().any(|item| {
        let panel = state.workspace.panel(item.request.panel);

        item.request.directory == panel.directory
            && match &item.result {
                Ok(snapshot) => {
                    snapshot.directory != panel.directory || snapshot.entries != panel.all_entries
                }
                Err(_) => false,
            }
    }) || state.workspace.preview_visible
        && result.changed_paths.iter().any(|path| {
            state.selected_path().is_some_and(|selected| {
                path == selected
                    || state
                        .selected_entry()
                        .is_some_and(|entry| entry.is_directory && path.parent() == Some(selected))
            })
        })
}
