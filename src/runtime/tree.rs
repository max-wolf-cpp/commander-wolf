use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};

use crate::app::Action;

enum Message {
    Expand(u64, PathBuf, bool),
    Shutdown,
}

pub(super) struct TreeRuntime {
    sender: mpsc::Sender<Message>,
    receiver: mpsc::Receiver<(u64, Action)>,
    latest: Arc<AtomicU64>,
    pending: Option<u64>,
    handle: Option<JoinHandle<()>>,
}

impl TreeRuntime {
    pub fn new() -> Self {
        let (sender, requests) = mpsc::channel();
        let (results, receiver) = mpsc::channel();
        let latest = Arc::new(AtomicU64::new(0));
        let worker_latest = Arc::clone(&latest);
        let handle = thread::spawn(move || expand_requests(requests, results, &worker_latest));

        Self {
            sender,
            receiver,
            latest,
            pending: None,
            handle: Some(handle),
        }
    }

    pub fn start(&mut self, generation: u64, root: PathBuf, hidden: bool) {
        self.latest.store(generation, Ordering::Release);
        self.pending = Some(generation);
        let _ = self.sender.send(Message::Expand(generation, root, hidden));
    }

    pub fn synchronize(&mut self, open: bool, generation: u64) {
        if let Some(pending) = self.pending
            && (!open || pending != generation)
        {
            self.latest.fetch_add(1, Ordering::AcqRel);
            self.pending = None;
        }
    }

    pub fn poll(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok((generation, action)) if self.pending == Some(generation) => {
                    self.pending = None;
                    actions.push(action);
                }
                Ok(_) => {}
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if let Some(generation) = self.pending.take() {
                        actions.push(Action::TreeLoadFailed {
                            generation,
                            message: "Tree worker disconnected".into(),
                        });
                    }
                    break;
                }
            }
        }

        actions
    }
}

impl Drop for TreeRuntime {
    fn drop(&mut self) {
        self.latest.fetch_add(1, Ordering::AcqRel);
        let _ = self.sender.send(Message::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn expand_requests(
    requests: mpsc::Receiver<Message>,
    results: mpsc::Sender<(u64, Action)>,
    latest: &AtomicU64,
) {
    while let Ok(Message::Expand(generation, root, hidden)) = requests.recv() {
        let cancelled = || latest.load(Ordering::Acquire) != generation;
        if cancelled() {
            continue;
        }

        let result = std::panic::catch_unwind(|| {
            crate::filesystem::tree::expand_all_bounded(&root, hidden, 10_000, &cancelled)
        })
        .unwrap_or_else(|_| Err("Tree worker panicked".into()));

        if cancelled() {
            continue;
        }

        let action = match result {
            Ok(entries) => Action::TreeExpandedAll {
                generation,
                entries,
            },
            Err(message) => Action::TreeLoadFailed {
                generation,
                message,
            },
        };

        let _ = results.send((generation, action));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn latest_tree_request_wins_and_close_cancels_without_leaking_worker() {
        let temp = TempDirectory::new();
        std::fs::write(temp.join("leaf"), "data").unwrap();

        let mut runtime = TreeRuntime::new();
        runtime.start(1, temp.join("missing"), true);
        runtime.start(2, temp.0.clone(), true);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let actions = runtime.poll();
            if !actions.is_empty() {
                assert!(
                    matches!(actions.as_slice(), [Action::TreeExpandedAll { generation: 2, entries }] if entries.len() == 1)
                );
                break;
            }

            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }

        runtime.start(3, temp.0.clone(), true);
        runtime.synchronize(false, 3);
        assert!(runtime.pending.is_none());
        assert!(runtime.poll().is_empty());

        // Drop joins the single persistent worker even when a request is queued.
        drop(runtime);
    }
}
