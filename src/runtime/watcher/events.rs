use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::time::Instant;

use notify::EventKind;

use super::{DirectoryWatcher, MAX_CHANGED_PATHS, WatchEvent};

impl DirectoryWatcher {
    pub(super) fn collect_events(&mut self, now: Instant) {
        if self.overflowed.swap(false, Ordering::Relaxed) {
            self.pending_paths.clear();
            self.pending_all = true;
            self.schedule(now);
        }

        while let Ok(event) = self.events.try_recv() {
            match event {
                Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                    if event.paths.is_empty() {
                        self.pending_paths.clear();
                        self.pending_all = true;
                        self.schedule(now);
                    }

                    for path in event.paths {
                        self.record_event_path(path, now);
                    }
                }
                Err(error) => {
                    self.pending_paths.clear();
                    self.pending_all = true;
                    self.schedule(now);

                    let message = error.to_string();
                    if self.reported_event_error.as_deref() != Some(&message) {
                        self.reported_event_error = Some(message.clone());
                        self.pending_notice = Some(format!(
                            "Directory watcher event failed: {message}; reconciling panel contents"
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    fn record_event_path(&mut self, path: PathBuf, now: Instant) {
        let aliased_paths = self
            .watch_aliases
            .iter()
            .filter_map(|(alias, resolved)| {
                if path == *resolved {
                    Some(alias.clone())
                } else if path.parent() == Some(resolved.as_path()) {
                    path.file_name().map(|name| alias.join(name))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for path in std::iter::once(path).chain(aliased_paths) {
            if event_affects(&path, &self.directories, self.preview_directory.as_deref()) {
                self.record_path(path, now);
            }
        }
    }

    pub(super) fn record_path(&mut self, path: PathBuf, now: Instant) {
        if !self.pending_all {
            if self.pending_paths.len() >= MAX_CHANGED_PATHS {
                self.pending_paths.clear();
                self.pending_all = true;
            } else {
                self.pending_paths.insert(path);
            }
        }

        self.schedule(now);
    }
}

pub(super) fn event_handler(
    sender: SyncSender<WatchEvent>,
    overflowed: Arc<AtomicBool>,
) -> impl FnMut(WatchEvent) + Send + 'static {
    move |event| {
        if sender.try_send(event).is_err() {
            overflowed.store(true, Ordering::Relaxed);
        }
    }
}

pub(super) fn event_affects(
    path: &Path,
    directories: &[PathBuf; 2],
    preview: Option<&Path>,
) -> bool {
    directories
        .iter()
        .filter(|directory| !directory.as_os_str().is_empty())
        .any(|directory| path == directory || path.parent() == Some(directory.as_path()))
        || preview.is_some_and(|directory| path == directory || path.parent() == Some(directory))
}
