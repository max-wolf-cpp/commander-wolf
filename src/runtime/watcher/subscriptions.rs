use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

use notify::RecursiveMode;

use crate::app::{Action, Notification};

use super::{DirectoryWatcher, RETRY_WATCH};

impl DirectoryWatcher {
    pub(super) fn synchronize_subscriptions(&mut self) -> Option<Action> {
        let mut desired = BTreeSet::new();
        for directory in self
            .directories
            .iter()
            .chain(self.preview_directory.iter())
            .filter(|path| !path.as_os_str().is_empty())
        {
            desired.insert(directory.clone());
            if let Some(parent) = directory.parent() {
                desired.insert(parent.to_path_buf());
            }
        }

        let now = Instant::now();
        let mut warning = None;
        self.unwatch_obsolete(&desired, now, &mut warning);

        self.failed
            .retain(|path, _| desired.contains(path) || self.watched.contains(path));
        self.reported_failures
            .retain(|path| desired.contains(path) || self.watched.contains(path));

        self.watch_missing(&desired, now, &mut warning);

        warning
    }

    fn unwatch_obsolete(
        &mut self,
        desired: &BTreeSet<PathBuf>,
        now: Instant,
        warning: &mut Option<Action>,
    ) {
        for path in self
            .watched
            .difference(desired)
            .cloned()
            .collect::<Vec<_>>()
        {
            if self.failed.get(&path).is_some_and(|retry| now < *retry) {
                continue;
            }

            match self.watcher.unwatch(&path) {
                Ok(()) => {
                    self.watched.remove(&path);
                    self.failed.remove(&path);
                    self.reported_failures.remove(&path);
                }
                Err(error) => {
                    self.failed.insert(path.clone(), now + RETRY_WATCH);
                    if warning.is_none() && self.reported_failures.insert(path.clone()) {
                        *warning = Some(Action::Notify(Notification::error(format!(
                            "Cannot stop watching '{}': {error}",
                            path.display()
                        ))));
                    }
                }
            }
        }
    }

    fn watch_missing(
        &mut self,
        desired: &BTreeSet<PathBuf>,
        now: Instant,
        warning: &mut Option<Action>,
    ) {
        for path in desired
            .difference(&self.watched)
            .cloned()
            .collect::<Vec<_>>()
        {
            if self.failed.get(&path).is_some_and(|retry| now < *retry) {
                continue;
            }

            match self.watcher.watch(&path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.watched.insert(path.clone());
                    self.failed.remove(&path);
                    self.reported_failures.remove(&path);
                }
                Err(error) => {
                    self.failed.insert(path.clone(), now + RETRY_WATCH);
                    if warning.is_none() && self.reported_failures.insert(path.clone()) {
                        *warning = Some(Action::Notify(Notification::error(format!(
                            "Cannot watch '{}': {error}; use :refresh if changes are missed",
                            path.display()
                        ))));
                    }
                }
            }
        }
    }
}
