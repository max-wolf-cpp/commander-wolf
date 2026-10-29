use std::path::{Path, PathBuf};

use super::PanelRefreshRequest;
use crate::workspace::PanelId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileOperationKind {
    Copy,
    Move,
    Delete,
    Rename,
    CreateDirectory,
    CreateFile,
    CreateArchive,
    ExtractArchive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileOperation {
    Copy {
        source: PathBuf,
        destination: PathBuf,
        overwrite: bool,
    },
    Move {
        source: PathBuf,
        destination: PathBuf,
        overwrite: bool,
    },
    Delete {
        source: PathBuf,
    },
    Rename {
        source: PathBuf,
        destination: PathBuf,
    },
    CreateDirectory {
        destination: PathBuf,
    },
    CreateFile {
        destination: PathBuf,
    },
    CreateArchive {
        sources: Vec<PathBuf>,
        destination: PathBuf,
    },
    ExtractArchive {
        source: PathBuf,
        destination: PathBuf,
    },
}

impl FileOperation {
    pub fn kind(&self) -> FileOperationKind {
        match self {
            Self::Copy { .. } => FileOperationKind::Copy,
            Self::Move { .. } => FileOperationKind::Move,
            Self::Delete { .. } => FileOperationKind::Delete,
            Self::Rename { .. } => FileOperationKind::Rename,
            Self::CreateDirectory { .. } => FileOperationKind::CreateDirectory,
            Self::CreateFile { .. } => FileOperationKind::CreateFile,
            Self::CreateArchive { .. } => FileOperationKind::CreateArchive,
            Self::ExtractArchive { .. } => FileOperationKind::ExtractArchive,
        }
    }

    pub fn source(&self) -> Option<&Path> {
        match self {
            Self::Copy { source, .. }
            | Self::Move { source, .. }
            | Self::Delete { source }
            | Self::Rename { source, .. }
            | Self::ExtractArchive { source, .. } => Some(source),
            Self::CreateDirectory { .. } | Self::CreateFile { .. } | Self::CreateArchive { .. } => {
                None
            }
        }
    }

    pub fn conflict_destination(&self) -> Option<&Path> {
        match self {
            Self::Copy { destination, .. } | Self::Move { destination, .. } => Some(destination),
            _ => None,
        }
    }

    pub fn overwrites(&self) -> bool {
        matches!(
            self,
            Self::Copy {
                overwrite: true,
                ..
            } | Self::Move {
                overwrite: true,
                ..
            }
        )
    }

    pub fn enable_overwrite(&mut self) {
        match self {
            Self::Copy { overwrite, .. } | Self::Move { overwrite, .. } => *overwrite = true,
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationRequest {
    pub operation: FileOperation,
    pub protected_directory: PathBuf,
    pub refresh: [PanelRefreshRequest; 2],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationBatchRequest {
    pub operations: Vec<FileOperation>,
    pub source_panel: PanelId,
    pub protected_directory: PathBuf,
    pub refresh: [PanelRefreshRequest; 2],
}

impl FileOperationBatchRequest {
    pub fn kind(&self) -> Option<FileOperationKind> {
        self.operations.first().map(FileOperation::kind)
    }

    pub fn is_delete_confirmation(&self) -> bool {
        !self.operations.is_empty()
            && self
                .operations
                .iter()
                .all(|operation| matches!(operation, FileOperation::Delete { .. }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationStatus {
    Success,
    Failure,
    Partial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationOutcome {
    pub status: OperationStatus,
    pub filesystem_changed: bool,
    pub message: String,
}

impl FileOperationOutcome {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: OperationStatus::Success,
            filesystem_changed: true,
            message: message.into(),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            status: OperationStatus::Failure,
            filesystem_changed: false,
            message: message.into(),
        }
    }

    pub fn changed_failure(message: impl Into<String>) -> Self {
        Self {
            status: OperationStatus::Partial,
            filesystem_changed: true,
            message: message.into(),
        }
    }
}
