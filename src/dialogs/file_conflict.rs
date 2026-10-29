use std::collections::HashSet;
use std::path::PathBuf;

use crate::file_operations::FileOperationBatchRequest;
use crate::preview::FileMetadata;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileConflict {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub new_metadata: FileMetadata,
    pub existing_metadata: FileMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileConflictState {
    pub request: FileOperationBatchRequest,
    pub conflicts: Vec<FileConflict>,
    pub current: usize,
    pub apply_to_all: bool,
    overwrite: HashSet<PathBuf>,
    skipped: HashSet<PathBuf>,
}

impl FileConflictState {
    pub fn new(request: FileOperationBatchRequest, conflicts: Vec<FileConflict>) -> Self {
        Self {
            request,
            conflicts,
            current: 0,
            apply_to_all: false,
            overwrite: HashSet::new(),
            skipped: HashSet::new(),
        }
    }

    pub fn toggle_apply_to_all(&mut self) {
        if self.is_batch() {
            self.apply_to_all = !self.apply_to_all;
        }
    }

    pub fn resolve_current(&mut self, overwrite: bool) -> bool {
        let end = if self.apply_to_all {
            self.conflicts.len()
        } else {
            (self.current + 1).min(self.conflicts.len())
        };

        for conflict in &self.conflicts[self.current..end] {
            if overwrite {
                self.overwrite.insert(conflict.destination.clone());
            } else {
                self.skipped.insert(conflict.destination.clone());
            }
        }

        self.current = end;

        self.current >= self.conflicts.len()
    }

    pub fn into_resolved_request(mut self) -> FileOperationBatchRequest {
        self.request.operations.retain(|operation| {
            operation
                .conflict_destination()
                .is_none_or(|destination| !self.skipped.contains(destination))
        });

        for operation in &mut self.request.operations {
            if operation
                .conflict_destination()
                .is_some_and(|destination| self.overwrite.contains(destination))
            {
                operation.enable_overwrite();
            }
        }

        self.request
    }

    pub fn current(&self) -> Option<&FileConflict> {
        self.conflicts.get(self.current)
    }

    pub fn is_batch(&self) -> bool {
        self.request.operations.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_operations::{FileOperation, PanelRefreshRequest, RefreshSelection};
    use crate::workspace::PanelId;
    use std::path::Path;

    fn metadata(path: &str) -> FileMetadata {
        FileMetadata {
            path: path.into(),
            file_type: "Regular file".into(),
            size: "1 B (1 bytes)".into(),
            created: "Unavailable".into(),
            modified: "Unavailable".into(),
            permissions: "Read/write".into(),
            extension: None,
            hard_links: None,
        }
    }

    fn request() -> FileOperationBatchRequest {
        let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
            panel,
            directory: PathBuf::from("."),
            selection: RefreshSelection {
                preferred: None,
                fallback_index: 0,
            },
        });

        FileOperationBatchRequest {
            operations: ["a", "b"]
                .into_iter()
                .map(|name| FileOperation::Copy {
                    source: PathBuf::from(format!("from/{name}")),
                    destination: PathBuf::from(format!("to/{name}")),
                    overwrite: false,
                })
                .collect(),
            source_panel: PanelId::Left,
            protected_directory: PathBuf::from("to"),
            refresh,
        }
    }

    #[test]
    fn apply_to_all_resolves_every_remaining_conflict() {
        let conflicts = ["a", "b"]
            .into_iter()
            .map(|name| FileConflict {
                source: PathBuf::from(format!("from/{name}")),
                destination: PathBuf::from(format!("to/{name}")),
                new_metadata: metadata(name),
                existing_metadata: metadata(name),
            })
            .collect();
        let mut state = FileConflictState::new(request(), conflicts);
        state.toggle_apply_to_all();

        assert!(state.resolve_current(true));
        let request = state.into_resolved_request();
        assert!(request.operations.iter().all(FileOperation::overwrites));
    }

    #[test]
    fn skipped_conflicts_are_removed_without_discarding_other_operations() {
        let conflicts = vec![FileConflict {
            source: PathBuf::from("from/a"),
            destination: PathBuf::from("to/a"),
            new_metadata: metadata("a"),
            existing_metadata: metadata("a"),
        }];
        let mut state = FileConflictState::new(request(), conflicts);

        assert!(state.resolve_current(false));
        let request = state.into_resolved_request();
        assert_eq!(request.operations.len(), 1);
        assert_eq!(
            request.operations[0].conflict_destination(),
            Some(Path::new("to/b"))
        );
    }
}
