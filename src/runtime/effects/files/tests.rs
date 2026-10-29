use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::*;
use crate::file_operations::{
    FileOperation, FileOperationKind, OperationStatus, PanelRefreshRequest, RefreshSelection,
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "commander-wolf-archive-worker-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn archive_creation_and_extraction_are_launched_as_background_operations() {
    let root = TestDirectory::new();
    let source = root.0.join("data.txt");
    fs::write(&source, vec![b'x'; 1024 * 1024]).unwrap();
    let archive = root.0.join("archive.zip");
    let output = root.0.join("output");
    let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: root.0.clone(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    });
    let mut runtime = OperationsRuntime::default();

    let batch = FileOperationBatchRequest {
        operations: vec![FileOperation::CreateArchive {
            sources: vec![source],
            destination: archive.clone(),
        }],
        source_panel: PanelId::Left,
        protected_directory: root.0.clone(),
        refresh: refresh.clone(),
    };
    let actions = perform_batch(batch, &mut runtime).unwrap();
    assert!(
        matches!(actions.as_slice(), [Action::FileOperationProgressStarted(progress)] if progress.kind == FileOperationKind::CreateArchive)
    );
    let actions = wait_for_completion(&mut runtime);
    assert!(actions.iter().any(|action| matches!(action, Action::FileOperationBatchCompleted { outcome, .. } if outcome.status == OperationStatus::Success)), "{actions:?}");

    let request = FileOperationRequest {
        operation: FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
        protected_directory: root.0.clone(),
        refresh,
    };
    let actions = perform_operation(request, &mut runtime).unwrap();
    assert!(
        matches!(actions.as_slice(), [Action::FileOperationProgressStarted(progress)] if progress.kind == FileOperationKind::ExtractArchive)
    );
    let actions = wait_for_completion(&mut runtime);
    assert!(actions.iter().any(|action| matches!(action, Action::FileOperationCompleted { outcome, .. } if outcome.status == OperationStatus::Success)), "{actions:?}");
    assert_eq!(
        fs::metadata(output.join("data.txt")).unwrap().len(),
        1024 * 1024
    );
}

fn wait_for_completion(runtime: &mut OperationsRuntime) -> Vec<Action> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut actions = Vec::new();
    loop {
        actions.extend(runtime.poll());
        if actions.iter().any(|action| {
            matches!(
                action,
                Action::FileOperationCompleted { .. } | Action::FileOperationBatchCompleted { .. }
            )
        }) {
            return actions;
        }
        assert!(Instant::now() < deadline, "archive worker did not complete");
        std::thread::sleep(Duration::from_millis(5));
    }
}
