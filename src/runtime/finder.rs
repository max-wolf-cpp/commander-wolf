use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use crate::app::Action;
use crate::finder::{FinderSearchRequest, FinderWorkerMessage};

struct Worker {
    cancellation: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

pub struct FinderRuntime {
    sender: mpsc::Sender<FinderWorkerMessage>,
    receiver: mpsc::Receiver<FinderWorkerMessage>,
    workers: Vec<Worker>,
}

impl FinderRuntime {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            sender,
            receiver,
            workers: Vec::new(),
        }
    }

    pub fn start(&mut self, request: FinderSearchRequest) {
        self.cancel();

        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let sender = self.sender.clone();
        let handle = thread::spawn(move || run_search(request, &worker_cancellation, &sender));

        self.workers.push(Worker {
            cancellation,
            handle,
        });
    }

    pub fn cancel(&self) {
        for worker in &self.workers {
            worker.cancellation.store(true, Ordering::Relaxed);
        }
    }

    pub fn poll(&mut self) -> Vec<Action> {
        let actions = self.receiver.try_iter().map(finder_action).collect();

        let mut index = 0;
        while index < self.workers.len() {
            if self.workers[index].handle.is_finished() {
                let worker = self.workers.swap_remove(index);
                let _ = worker.handle.join();
            } else {
                index += 1;
            }
        }

        actions
    }

    pub fn shutdown(&mut self) {
        self.cancel();
        for worker in self.workers.drain(..) {
            let _ = worker.handle.join();
        }
    }
}

impl Drop for FinderRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn finder_action(message: FinderWorkerMessage) -> Action {
    match message {
        FinderWorkerMessage::ResultsAdded {
            generation,
            results,
        } => Action::FinderResultsAdded {
            generation,
            results,
        },
        FinderWorkerMessage::Completed {
            generation,
            skipped,
            truncated,
            cancelled,
        } => Action::FinderCompleted {
            generation,
            skipped,
            truncated,
            cancelled,
        },
        FinderWorkerMessage::Failed {
            generation,
            message,
        } => Action::FinderFailed {
            generation,
            message,
        },
    }
}

fn run_search(
    request: FinderSearchRequest,
    cancellation: &AtomicBool,
    sender: &mpsc::Sender<FinderWorkerMessage>,
) {
    let generation = request.generation;
    let outcome = crate::filesystem::finder::walk(
        &request.root,
        &request.matcher,
        request.options,
        cancellation,
        |results| {
            sender
                .send(FinderWorkerMessage::ResultsAdded {
                    generation,
                    results,
                })
                .is_ok()
        },
    );

    let message = match outcome {
        Ok(outcome) => FinderWorkerMessage::Completed {
            generation,
            skipped: outcome.skipped,
            truncated: outcome.truncated,
            cancelled: outcome.cancelled,
        },
        Err(message) => FinderWorkerMessage::Failed {
            generation,
            message,
        },
    };

    let _ = sender.send(message);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Barrier, mpsc};

    use super::*;
    use crate::finder::{FinderOptions, GlobMatcher};

    #[test]
    fn shutdown_joins_workers_without_leaking_threads() {
        let mut runtime = FinderRuntime::new();
        runtime.start(FinderSearchRequest {
            generation: 1,
            root: PathBuf::from("missing-root"),
            matcher: GlobMatcher::parse("*").unwrap(),
            options: FinderOptions::default(),
        });
        runtime.shutdown();
        assert!(runtime.workers.is_empty());
    }

    #[test]
    fn cancellation_and_shutdown_are_coordinated_without_timing_sleeps() {
        let mut runtime = FinderRuntime::new();

        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let gate = Arc::new(Barrier::new(2));
        let worker_gate = Arc::clone(&gate);
        let (started_sender, started_receiver) = mpsc::channel();
        let (finished_sender, finished_receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            started_sender.send(()).unwrap();
            worker_gate.wait();
            assert!(worker_cancellation.load(Ordering::Relaxed));
            finished_sender.send(()).unwrap();
        });
        runtime.workers.push(Worker {
            cancellation,
            handle,
        });

        started_receiver.recv().unwrap();
        runtime.cancel();
        gate.wait();
        runtime.shutdown();

        finished_receiver.recv().unwrap();
        assert!(runtime.workers.is_empty());
    }
}
