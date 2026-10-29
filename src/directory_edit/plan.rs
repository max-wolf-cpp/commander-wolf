use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{DirectoryEditState, EditLine};
use crate::file_operations::{FileOperation, PanelRefreshRequest, validate_new_name};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEditRequest {
    pub operations: Vec<FileOperation>,
    pub protected_directory: PathBuf,
    pub refresh: [PanelRefreshRequest; 2],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryEditChange {
    Create { name: String },
    Rename { source: String, destination: String },
    Delete { name: String },
}

impl DirectoryEditState {
    pub fn request(
        &self,
        protected_directory: PathBuf,
        refresh: [PanelRefreshRequest; 2],
    ) -> Result<DirectoryEditRequest, String> {
        let mut operations = Vec::new();
        let mut retained = HashSet::new();
        let mut names = HashSet::new();

        for line in &self.lines {
            let name = line.text.strip_suffix('/').unwrap_or(&line.text);
            if name.is_empty() {
                continue;
            }

            validate_new_name(name)?;
            let key = if cfg!(windows) {
                name.to_lowercase()
            } else {
                name.to_owned()
            };
            if !names.insert(key) {
                return Err(format!("Duplicate edit-buffer name: '{name}'"));
            }

            if let Some(original) = &line.original {
                retained.insert(original.path.clone());
            }
            if let Some(operation) = line_operation(line, &self.directory, name) {
                operations.push(operation);
            }
        }

        let mut deletes = self
            .originals
            .iter()
            .filter(|original| !retained.contains(&original.path))
            .map(|original| FileOperation::Delete {
                source: original.path.clone(),
            })
            .collect::<Vec<_>>();
        deletes.extend(operations);

        Ok(DirectoryEditRequest {
            operations: deletes,
            protected_directory,
            refresh,
        })
    }

    pub fn changes(&self) -> Vec<DirectoryEditChange> {
        let mut changes = Vec::new();
        let mut retained = HashSet::new();

        for line in &self.lines {
            let name = line.text.strip_suffix('/').unwrap_or(&line.text);
            if name.is_empty() {
                continue;
            }

            if let Some(original) = &line.original {
                retained.insert(original.path.clone());
                if name != original.name {
                    changes.push(DirectoryEditChange::Rename {
                        source: original.name.clone(),
                        destination: name.to_owned(),
                    });
                }
            } else {
                changes.push(DirectoryEditChange::Create {
                    name: name.to_owned(),
                });
            }
        }

        changes.extend(
            self.originals
                .iter()
                .filter(|original| !retained.contains(&original.path))
                .map(|original| DirectoryEditChange::Delete {
                    name: original.name.clone(),
                }),
        );
        changes.sort_by_key(|change| match change {
            DirectoryEditChange::Create { .. } => 0,
            DirectoryEditChange::Rename { .. } => 1,
            DirectoryEditChange::Delete { .. } => 2,
        });

        changes
    }

    pub fn has_changes(&self) -> bool {
        let mut retained = HashSet::new();

        for line in &self.lines {
            let name = line.text.strip_suffix('/').unwrap_or(&line.text);
            if name.is_empty() {
                continue;
            }

            match &line.original {
                Some(original) => {
                    retained.insert(original.path.clone());
                    if name != original.name {
                        return true;
                    }
                }
                None => return true,
            }
        }

        self.originals
            .iter()
            .any(|original| !retained.contains(&original.path))
    }
}

fn line_operation(line: &EditLine, directory: &Path, name: &str) -> Option<FileOperation> {
    let destination = directory.join(name);

    match &line.original {
        Some(original) if name != original.name => Some(FileOperation::Rename {
            source: original.path.clone(),
            destination,
        }),
        Some(_) => None,
        None if line.text.ends_with('/') => Some(FileOperation::CreateDirectory { destination }),
        None => Some(FileOperation::CreateFile { destination }),
    }
}
