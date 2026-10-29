use std::path::{Path, PathBuf};

use super::FileOperationKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    pub kind: FileOperationKind,
    pub path: PathBuf,
    pub phase: String,
    pub file_done: u64,
    pub file_total: u64,
    pub total_done: u64,
    pub total: u64,
    pub cancelling: bool,
}

impl OperationProgress {
    pub fn new(kind: FileOperationKind) -> Self {
        Self {
            kind,
            path: PathBuf::new(),
            phase: "Preparing".into(),
            file_done: 0,
            file_total: 0,
            total_done: 0,
            total: 0,
            cancelling: false,
        }
    }
}

// Filesystem mechanisms report progress without depending on UI or worker ownership.
pub trait OperationObserver {
    fn check_cancelled(&self) -> Result<(), String> {
        Ok(())
    }
    fn begin_file(&mut self, _path: &Path, _size: u64) {}
    fn set_total(&mut self, _size: u64) {}
    fn advance(&mut self, _bytes: u64) {}
    fn finish_file(&mut self) {}
    fn phase(&mut self, _phase: &str) {}
}

pub struct NoopObserver;
impl OperationObserver for NoopObserver {}
