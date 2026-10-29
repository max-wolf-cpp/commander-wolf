mod editing;
mod plan;

use std::path::{Path, PathBuf};

use crate::workspace::{DirectoryEntry, PanelId, ViewportState};

pub use plan::{DirectoryEditChange, DirectoryEditRequest};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditOriginal {
    pub is_hidden: bool,
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditLine {
    pub text: String,
    pub original: Option<EditOriginal>,
}

impl EditLine {
    pub fn display_text(&self) -> String {
        if self
            .original
            .as_ref()
            .is_some_and(|entry| entry.is_directory)
            && !self.text.ends_with('/')
        {
            format!("{}/", self.text)
        } else {
            self.text.clone()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEditState {
    pub active: bool,
    pub awaiting_confirmation: bool,
    pub panel: PanelId,
    pub directory: PathBuf,
    pub lines: Vec<EditLine>,
    pub originals: Vec<EditOriginal>,
    pub cursor_row: usize,
    pub cursor_column: usize,
    pub viewport: ViewportState,
}

impl Default for DirectoryEditState {
    fn default() -> Self {
        Self {
            active: false,
            awaiting_confirmation: false,
            panel: PanelId::Left,
            directory: PathBuf::new(),
            lines: Vec::new(),
            originals: Vec::new(),
            cursor_row: 0,
            cursor_column: 0,
            viewport: ViewportState::default(),
        }
    }
}

impl DirectoryEditState {
    pub fn begin(
        &mut self,
        panel: PanelId,
        directory: PathBuf,
        entries: &[DirectoryEntry],
        selected: usize,
    ) {
        let editable = editable_lines(entries);
        let originals = editable
            .iter()
            .filter_map(|line| line.original.clone())
            .collect();

        let parent_offset = entries.first().is_some_and(|entry| entry.is_parent) as usize;
        let visible_rows = self.viewport.visible_rows;
        *self = Self {
            active: true,
            panel,
            directory,
            lines: if editable.is_empty() {
                vec![EditLine {
                    text: String::new(),
                    original: None,
                }]
            } else {
                editable
            },
            originals,
            cursor_row: selected.saturating_sub(parent_offset),
            viewport: ViewportState {
                visible_rows,
                ..ViewportState::default()
            },
            ..Self::default()
        };

        self.cursor_row = self.cursor_row.min(self.lines.len().saturating_sub(1));
        self.cursor_column = self.lines[self.cursor_row].text.chars().count();
        self.ensure_visible();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.viewport.visible_rows = rows;
        self.ensure_visible();
    }

    fn ensure_visible(&mut self) {
        self.viewport
            .ensure_visible(self.cursor_row, self.lines.len(), 3);
    }
}

pub fn relative_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn editable_lines(entries: &[DirectoryEntry]) -> Vec<EditLine> {
    entries
        .iter()
        .filter(|entry| !entry.is_parent)
        .map(|entry| {
            let original = EditOriginal {
                is_hidden: entry.is_hidden,
                path: entry.path.clone(),
                name: entry.name.clone(),
                is_directory: entry.is_directory,
            };
            EditLine {
                text: original.name.clone(),
                original: Some(original),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
