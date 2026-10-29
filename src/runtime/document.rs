use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use crate::app::Action;
use crate::preview::document::{DocumentOutput, DocumentRequest};

use self::worker::{WorkerMessage, WorkerResult};

mod cache;
mod worker;

pub struct DocumentRuntime {
    sender: mpsc::Sender<WorkerMessage>,
    receiver: mpsc::Receiver<WorkerResult>,
    latest_request: Arc<AtomicU64>,
    pending: RefCell<Option<DocumentRequest>>,
    handle: Option<JoinHandle<()>>,
}

impl DocumentRuntime {
    pub fn new() -> Self {
        Self::with_loader(crate::preview::document::load)
    }

    fn with_loader(
        loader: impl FnMut(&DocumentRequest) -> Result<DocumentOutput, String> + Send + 'static,
    ) -> Self {
        let (sender, requests) = mpsc::channel();
        let (results, receiver) = mpsc::channel();
        let latest_request = Arc::new(AtomicU64::new(0));
        let latest = Arc::clone(&latest_request);
        let handle = thread::spawn(move || worker::run(requests, results, &latest, loader));

        Self {
            sender,
            receiver,
            latest_request,
            pending: RefCell::new(None),
            handle: Some(handle),
        }
    }

    pub fn start(&self, request: DocumentRequest) {
        self.latest_request
            .store(request.request_id, Ordering::Release);
        *self.pending.borrow_mut() = Some(request.clone());

        // A failed send is reported by poll's disconnected-channel path.
        let _ = self.sender.send(WorkerMessage::Render(request));
    }

    pub fn cancel(&self) {
        self.latest_request.fetch_add(1, Ordering::AcqRel);
        self.pending.borrow_mut().take();
    }

    pub fn poll(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(result)
                    if self
                        .pending
                        .borrow()
                        .as_ref()
                        .is_some_and(|request| request.request_id == result.request.request_id) =>
                {
                    self.pending.borrow_mut().take();
                    actions.push(result.into_action());
                }
                Ok(_) => {}
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if let Some(request) = self.pending.borrow_mut().take() {
                        actions.push(
                            WorkerResult {
                                request,
                                output: Err("Document worker disconnected".into()),
                            }
                            .into_action(),
                        );
                    }
                    break;
                }
            }
        }

        actions
    }

    pub fn shutdown(&mut self) {
        self.cancel();
        let _ = self.sender.send(WorkerMessage::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DocumentRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests;
