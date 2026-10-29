use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use crate::app::{Action, Notification};
use crate::file_operations::progress::OperationProgress;
use crate::file_operations::{
    FileOperationBatchRequest, FileOperationOutcome, FileOperationRequest,
};

#[derive(Clone)]
pub(super) enum OperationJob {
    Single(FileOperationRequest),
    Batch(FileOperationBatchRequest),
    Edit(crate::directory_edit::DirectoryEditRequest),
}

#[derive(Default)]
pub(super) struct OperationsRuntime {
    running: Option<Running>,
}

struct Running {
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<Option<OperationProgress>>>,
    result: mpsc::Receiver<Action>,
    handle: JoinHandle<()>,
    job: OperationJob,
}

impl OperationsRuntime {
    pub fn start(&mut self, job: OperationJob) -> Vec<Action> {
        self.start_with(job, |job, cancel, notify| match job {
            OperationJob::Edit(request) => {
                crate::filesystem::edit::execute_with_progress(request, cancel, notify)
            }
            _ => crate::filesystem::execute_batch_with_progress(&job.batch(), cancel, notify),
        })
    }

    fn start_with(
        &mut self,
        job: OperationJob,
        worker: impl FnOnce(
            &OperationJob,
            &AtomicBool,
            &mut dyn FnMut(OperationProgress),
        ) -> FileOperationOutcome
        + Send
        + 'static,
    ) -> Vec<Action> {
        if self.running.is_some() {
            return vec![Action::Notify(Notification::error(
                "A file operation is already running",
            ))];
        }

        let request = job.batch();
        let Some(kind) = request.kind() else {
            return vec![job.completed(FileOperationOutcome::failure(
                "File operation batch is empty",
            ))];
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(None));
        let worker_cancel = Arc::clone(&cancel);
        let worker_progress = Arc::clone(&progress);
        let (sender, result) = mpsc::channel();
        let worker_job = job.clone();
        let handle = thread::Builder::new()
            .name("file-operation".into())
            .spawn(move || {
                let execute = || {
                    let mut notify = |update| {
                        let mut latest = worker_progress
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        *latest = Some(update);
                    };
                    worker(&worker_job, &worker_cancel, &mut notify)
                };
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(execute))
                    .unwrap_or_else(|_| FileOperationOutcome::changed_failure(
                        "File operation worker failed; check the affected paths for partial changes",
                    ));
                let _ = sender.send(worker_job.completed(outcome));
            });

        match handle {
            Ok(handle) => {
                self.running = Some(Running {
                    cancel,
                    progress,
                    result,
                    handle,
                    job,
                });
                vec![Action::FileOperationProgressStarted(
                    OperationProgress::new(kind),
                )]
            }
            Err(error) => vec![job.completed(FileOperationOutcome::failure(format!(
                "Cannot start file operation worker: {error}"
            )))],
        }
    }

    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn poll(&mut self) -> Vec<Action> {
        let Some(running) = &self.running else {
            return Vec::new();
        };

        let mut actions = Vec::new();
        if let Some(progress) = running
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            actions.push(Action::FileOperationProgressUpdated(progress));
        }

        let outcome = match running.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(running.job.clone().completed(
                FileOperationOutcome::changed_failure(
                    "File operation worker disconnected; check the affected paths",
                ),
            )),
        };

        if let Some(outcome) = outcome {
            let running = self.running.take().unwrap();
            let _ = running.handle.join();
            actions.push(outcome);
        }

        actions
    }
}

impl OperationJob {
    fn batch(&self) -> FileOperationBatchRequest {
        match self {
            Self::Single(request) => FileOperationBatchRequest {
                operations: vec![request.operation.clone()],
                source_panel: request.refresh[0].panel,
                protected_directory: request.protected_directory.clone(),
                refresh: request.refresh.clone(),
            },
            Self::Batch(request) => request.clone(),
            Self::Edit(request) => FileOperationBatchRequest {
                operations: request.operations.clone(),
                source_panel: request.refresh[0].panel,
                protected_directory: request.protected_directory.clone(),
                refresh: request.refresh.clone(),
            },
        }
    }

    fn completed(self, outcome: FileOperationOutcome) -> Action {
        match self {
            Self::Single(request) => Action::FileOperationCompleted { request, outcome },
            Self::Batch(request) => Action::FileOperationBatchCompleted { request, outcome },
            Self::Edit(request) => Action::DirectoryEditCompleted {
                results: Box::new(crate::filesystem::refresh_panels(request.refresh.clone())),
                request,
                outcome,
            },
        }
    }
}

impl Drop for OperationsRuntime {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            running.cancel.store(true, Ordering::Relaxed);
            let _ = running.handle.join();
        }
    }
}

#[cfg(test)]
mod tests;
