use super::*;
use crate::file_operations::{
    FileOperation, FileOperationKind, OperationStatus, PanelRefreshRequest, RefreshSelection,
};
use crate::test_support::TempDirectory;
use crate::workspace::PanelId;
use std::time::{Duration, Instant};

fn job(root: &std::path::Path) -> OperationJob {
    OperationJob::Single(FileOperationRequest {
        operation: FileOperation::CreateFile {
            destination: root.join("created"),
        },
        protected_directory: root.to_owned(),
        refresh: [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
            panel,
            directory: root.to_owned(),
            selection: RefreshSelection {
                preferred: None,
                fallback_index: 0,
            },
        }),
    })
}

fn finish(runtime: &mut OperationsRuntime) -> Vec<Action> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut actions = Vec::new();
    while runtime.running.is_some() {
        actions.extend(runtime.poll());
        assert!(Instant::now() < deadline, "operation did not finish");
        thread::yield_now();
    }

    assert!(runtime.poll().is_empty());

    actions
}

#[test]
fn runtime_delivers_progress_rejects_reentry_and_joins_one_result() {
    let root = TempDirectory::new();
    let mut runtime = OperationsRuntime::default();
    let (sent, ready) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let started = runtime.start_with(job(&root.0), move |_, _, notify| {
        let mut progress = OperationProgress::new(FileOperationKind::CreateFile);
        progress.total_done = 7;
        notify(progress);
        sent.send(()).unwrap();
        gate.recv_timeout(Duration::from_secs(5)).unwrap();
        FileOperationOutcome::success("done")
    });
    assert!(matches!(
        started.as_slice(),
        [Action::FileOperationProgressStarted(_)]
    ));

    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(
        runtime.start(job(&root.0)).as_slice(),
        [Action::Notify(_)]
    ));
    assert!(
        matches!(runtime.poll().as_slice(), [Action::FileOperationProgressUpdated(progress)] if progress.total_done == 7)
    );

    release.send(()).unwrap();
    let actions = finish(&mut runtime);
    assert!(
        matches!(actions.as_slice(), [Action::FileOperationCompleted { outcome, .. }] if outcome.status == OperationStatus::Success)
    );
}

#[test]
fn cancellation_before_execution_and_drop_during_execution_join_workers() {
    let root = TempDirectory::new();
    let mut runtime = OperationsRuntime::default();
    let (release, gate) = mpsc::channel();
    runtime.start_with(job(&root.0), move |job, cancel, notify| {
        gate.recv_timeout(Duration::from_secs(5)).unwrap();
        crate::filesystem::execute_batch_with_progress(&job.batch(), cancel, notify)
    });
    runtime.cancel();

    release.send(()).unwrap();
    let actions = finish(&mut runtime);
    assert!(actions.iter().any(|action| matches!(action, Action::FileOperationCompleted { outcome, .. } if outcome.status == OperationStatus::Failure && outcome.message.contains("Cancelled"))));
    assert!(!root.join("created").exists());

    let (entered, ready) = mpsc::channel();
    let (exited, done) = mpsc::channel();
    runtime.start_with(job(&root.0), move |_, cancel, _| {
        entered.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !cancel.load(Ordering::Relaxed) {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        exited.send(()).unwrap();
        FileOperationOutcome::changed_failure("Cancelled after partial changes")
    });

    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(runtime);
    done.try_recv()
        .expect("Drop returned before the worker exited");
}

#[test]
fn panic_and_disconnected_worker_produce_one_partial_failure() {
    let root = TempDirectory::new();
    let mut runtime = OperationsRuntime::default();
    runtime.start_with(job(&root.0), |_, _, _| panic!("controlled worker panic"));
    let actions = finish(&mut runtime);
    assert!(
        matches!(actions.as_slice(), [Action::FileOperationCompleted { outcome, .. }] if outcome.status == OperationStatus::Partial && outcome.filesystem_changed)
    );

    let (sender, result) = mpsc::channel();
    drop(sender);
    runtime.running = Some(Running {
        cancel: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(Mutex::new(None)),
        result,
        handle: thread::spawn(|| {}),
        job: job(&root.0),
    });
    assert!(
        matches!(finish(&mut runtime).as_slice(), [Action::FileOperationCompleted { outcome, .. }] if outcome.message.contains("disconnected"))
    );
}

#[test]
fn terminal_outcomes_clear_ui_progress_and_preserve_partial_refresh_message() {
    use crate::app::{AppState, Effect, update};
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;
    let root = TempDirectory::new();
    for outcome in [
        FileOperationOutcome::success("success"),
        FileOperationOutcome::failure("failed"),
        FileOperationOutcome::failure("Cancelled"),
        FileOperationOutcome::changed_failure("Cancelled after partial changes"),
    ] {
        let mut state = AppState::new(
            root.0.clone(),
            "test".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            WorkspaceOptions::default(),
        );
        update(
            &mut state,
            Action::FileOperationProgressStarted(OperationProgress::new(
                FileOperationKind::CreateFile,
            )),
        );
        let effects = update(&mut state, job(&root.0).completed(outcome.clone()));
        assert!(state.operation_progress.is_none());
        assert_eq!(
            state.notification.as_ref().unwrap().message,
            outcome.message
        );
        assert_eq!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::RefreshPanels { .. })),
            outcome.filesystem_changed
        );
    }
}
