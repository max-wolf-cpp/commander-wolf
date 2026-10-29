use std::path::PathBuf;

use super::{FileOperationKind, FileOperationOutcome};
use crate::workspace::{DirectorySnapshot, PanelId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshSelection {
    pub preferred: Option<PathBuf>,
    pub fallback_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelRefreshRequest {
    pub panel: PanelId,
    pub directory: PathBuf,
    pub selection: RefreshSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelRefreshResult {
    pub request: PanelRefreshRequest,
    pub result: Result<DirectorySnapshot, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefreshContext {
    Manual,
    Operation {
        kind: FileOperationKind,
        outcome: FileOperationOutcome,
    },
}
