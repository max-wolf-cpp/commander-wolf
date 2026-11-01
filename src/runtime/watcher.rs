use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use notify::{Config, Event, Watcher};

use crate::app::{Action, AppState, Notification};
use crate::workspace::PanelId;

use self::events::event_handler;
use self::refresh::{RefreshResult, can_apply};

mod events;
mod refresh;
mod subscriptions;

const DEBOUNCE: Duration = Duration::from_millis(100);
const RECONCILE: Duration = Duration::from_secs(10);
const RETRY_WATCH: Duration = Duration::from_secs(5);
const EVENT_CAPACITY: usize = 256;
const MAX_CHANGED_PATHS: usize = 256;

type WatchEvent = notify::Result<Event>;

pub struct DirectoryWatcher {
    watcher: Box<dyn Watcher>,
    events: Receiver<WatchEvent>,
    overflowed: Arc<AtomicBool>,
    watched: BTreeSet<PathBuf>,
    watch_aliases: BTreeMap<PathBuf, PathBuf>,
    failed: BTreeMap<PathBuf, Instant>,
    reported_failures: BTreeSet<PathBuf>,
    refresh_errors: [Option<(PathBuf, String)>; 2],
    reported_event_error: Option<String>,
    pending_notice: Option<String>,
    directories: [PathBuf; 2],
    preview_directory: Option<PathBuf>,
    pending_paths: BTreeSet<PathBuf>,
    pending_all: bool,
    due_at: Option<Instant>,
    worker: Option<JoinHandle<RefreshResult>>,
    completed: Option<RefreshResult>,
    last_reconcile: Instant,
}

impl DirectoryWatcher {
    pub fn new() -> Result<Self, String> {
        let (sender, events) = mpsc::sync_channel(EVENT_CAPACITY);
        let overflowed = Arc::new(AtomicBool::new(false));
        let watcher: Box<dyn Watcher> =
            match notify::recommended_watcher(event_handler(sender.clone(), overflowed.clone())) {
                Ok(watcher) => Box::new(watcher),
                Err(native_error) => Box::new(
                    notify::PollWatcher::new(
                        event_handler(sender, overflowed.clone()),
                        Config::default().with_poll_interval(Duration::from_secs(2)),
                    )
                    .map_err(|poll_error| {
                        format!(
                            "cannot start directory watcher: {native_error}; polling fallback failed: {poll_error}"
                        )
                    })?,
                ),
            };

        Ok(Self {
            watcher,
            events,
            overflowed,
            watched: BTreeSet::new(),
            watch_aliases: BTreeMap::new(),
            failed: BTreeMap::new(),
            reported_failures: BTreeSet::new(),
            refresh_errors: [None, None],
            reported_event_error: None,
            pending_notice: None,
            directories: [PathBuf::new(), PathBuf::new()],
            preview_directory: None,
            pending_paths: BTreeSet::new(),
            pending_all: false,
            due_at: None,
            worker: None,
            completed: None,
            last_reconcile: Instant::now(),
        })
    }

    pub fn synchronize(&mut self, state: &AppState) -> Option<Action> {
        let directories = [PanelId::Left, PanelId::Right]
            .map(|panel| state.workspace.panel(panel).directory.clone());
        let preview_directory = state
            .workspace
            .preview_visible
            .then(|| state.selected_entry())
            .flatten()
            .filter(|entry| entry.is_directory && !entry.is_parent)
            .map(|entry| entry.path.clone());

        if self.directories != directories {
            self.directories = directories;
            self.completed = None;
            self.refresh_errors = [None, None];
            self.schedule(Instant::now());
        }
        self.preview_directory = preview_directory;

        self.synchronize_subscriptions()
    }

    pub fn poll(&mut self, state: &AppState) -> Option<Action> {
        let now = Instant::now();
        self.collect_events(now);
        if now.duration_since(self.last_reconcile) >= RECONCILE {
            self.last_reconcile = now;
            self.schedule(now);
        }

        if let Some(action) = self.collect_refresh(now) {
            return Some(action);
        }

        self.discard_stale_refresh(state, now);
        if can_apply(state)
            && let Some(action) = self.apply_refresh(state)
        {
            return Some(action);
        }

        if let Some(action) = self.start_refresh(state, now) {
            return Some(action);
        }

        self.pending_notice
            .take()
            .map(|message| Action::Notify(Notification::error(message)))
    }

    fn schedule(&mut self, now: Instant) {
        self.due_at.get_or_insert(now + DEBOUNCE);
    }
}

#[cfg(test)]
mod tests;
