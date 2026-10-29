use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::app::{Action, Notification};
use crate::workspace::{PanelId, WorkspaceState};

use self::format::{MAX_BYTES, decode, encode};

mod format;

pub(super) struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(state_directory: PathBuf) -> Self {
        Self {
            path: state_directory.join("session.dat"),
        }
    }

    pub fn startup_actions(&self, initial_path: &Path, restore: bool) -> Vec<Action> {
        if !restore {
            return vec![Action::Initialize];
        }

        let directories = match self.load() {
            Ok(Some(directories)) => directories,
            Ok(None) => return vec![Action::Initialize],
            Err(error) => {
                return vec![
                    Action::Initialize,
                    Action::Notify(Notification::error(error)),
                ];
            }
        };

        let mut actions = Vec::new();
        for (panel, directory) in [PanelId::Left, PanelId::Right].into_iter().zip(directories) {
            let loaded = if directory.is_dir() {
                crate::filesystem::load_directory(panel, &directory, None)
            } else {
                Err(format!(
                    "Saved directory '{}' is unavailable",
                    directory.display()
                ))
            };
            match loaded {
                Ok(action) => actions.push(action),
                Err(error) => {
                    actions.push(
                        crate::filesystem::load_directory(panel, initial_path, None)
                            .unwrap_or_else(|message| Action::DirectoryLoadFailed {
                                panel,
                                message,
                            }),
                    );
                    actions.push(Action::Notify(Notification::error(format!(
                        "{error}; using startup path for the {panel:?} panel"
                    ))));
                }
            }
        }

        actions
    }

    pub fn save(&self, enabled: bool, workspace: &WorkspaceState) -> Result<(), String> {
        if !enabled {
            return Ok(());
        }

        let bytes = encode(workspace)?;

        super::state_file::write(&self.path, &bytes)
            .map_err(|error| format!("Cannot save session '{}': {error}", self.path.display()))
    }

    fn load(&self) -> Result<Option<[PathBuf; 2]>, String> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Cannot read session '{}': {error}",
                    self.path.display()
                ));
            }
        };

        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("Cannot read session '{}': {error}", self.path.display()))?;

        decode(&bytes)
            .map(Some)
            .map_err(|error| format!("Invalid session '{}': {error}", self.path.display()))
    }
}

#[cfg(test)]
mod tests;
