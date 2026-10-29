use std::fs;
#[cfg(windows)]
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::file_operations::progress::{NoopObserver, OperationObserver};
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationRequest, OperationStatus,
    PanelRefreshRequest, RefreshSelection,
};
use crate::filesystem::faults;
use crate::workspace::PanelId;

use super::moving::move_path_with;
use super::removal::remove_source_observed;
use super::{execute_batch, execute_operation, execute_operation_observed};

mod cancellation;
mod copying;
mod entries;
mod links;
mod moving;
mod publication;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "commander-wolf-operations-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();

        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(operation: FileOperation, root: &Path) -> FileOperationRequest {
    let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: root.to_path_buf(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    });

    FileOperationRequest {
        operation,
        protected_directory: root.to_path_buf(),
        refresh,
    }
}

fn artifacts(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".commander-wolf-stage-")
        })
        .collect()
}
