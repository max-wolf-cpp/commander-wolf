use std::path::{Path, PathBuf};

use super::PanelState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    pub is_parent: bool,
    pub is_hidden: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectorySnapshot {
    pub directory: PathBuf,
    pub entries: Vec<DirectoryEntry>,
    pub selected: usize,
}

impl PanelState {
    pub fn replace_directory(
        &mut self,
        directory: PathBuf,
        entries: Vec<DirectoryEntry>,
        selected: usize,
        configured_scroll_offset: usize,
    ) {
        self.revision = self.revision.wrapping_add(1);
        self.directory = directory;
        self.entries = entries;
        self.marked.clear();

        self.selected = selected.min(self.entries.len().saturating_sub(1));
        self.all_entries = self.entries.clone();
        self.filter_hidden();

        self.viewport.scroll = 0;
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), configured_scroll_offset);
    }

    pub fn refresh_directory(
        &mut self,
        snapshot: DirectorySnapshot,
        configured_scroll_offset: usize,
    ) {
        self.revision = self.revision.wrapping_add(1);
        let previous_scroll = self.viewport.scroll;

        self.directory = snapshot.directory;
        self.entries = snapshot.entries;
        self.marked.retain(|path| {
            self.entries
                .iter()
                .any(|entry| !entry.is_parent && entry.path == *path)
        });

        self.selected = snapshot.selected.min(self.entries.len().saturating_sub(1));
        self.all_entries = self.entries.clone();
        self.filter_hidden();

        self.viewport.scroll = previous_scroll;
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), configured_scroll_offset);
    }

    pub fn set_show_hidden(&mut self, show: bool, scroll_offset: usize) {
        // Preserve edits to visible entries before rebuilding the filtered list.
        if !self.hide_hidden {
            self.all_entries = self.entries.clone();
        }

        let selected_path = self.selected_path().map(Path::to_path_buf);
        self.hide_hidden = !show;
        self.entries = self.all_entries.clone();

        self.selected = selected_path
            .as_ref()
            .and_then(|path| self.entries.iter().position(|entry| &entry.path == path))
            .unwrap_or(self.selected);
        self.filter_hidden();

        self.viewport
            .ensure_visible(self.selected, self.entries.len(), scroll_offset);
    }

    fn filter_hidden(&mut self) {
        let selected_path = self.selected_path().map(Path::to_path_buf);

        if self.hide_hidden {
            self.entries
                .retain(|entry| entry.is_parent || !entry.is_hidden);
        }

        self.selected = selected_path
            .as_ref()
            .and_then(|path| self.entries.iter().position(|entry| &entry.path == path))
            .unwrap_or_else(|| self.selected.min(self.entries.len().saturating_sub(1)));

        self.marked
            .retain(|path| self.entries.iter().any(|entry| &entry.path == path));
    }
}
