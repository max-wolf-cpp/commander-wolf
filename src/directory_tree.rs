use std::ops::Range;
use std::path::PathBuf;

use crate::workspace::{PanelId, SelectionMotion, ViewportState, apply_selection_motion};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeEntry {
    pub is_hidden: bool,
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_directory: bool,
    pub expanded: bool,
    pub loading: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryTreeState {
    pub open: bool,
    pub panel: PanelId,
    pub root: PathBuf,
    pub entries: Vec<TreeEntry>,
    pub selected: usize,
    pub viewport: ViewportState,
    pub generation: u64,
    pub error: Option<String>,
}

impl Default for DirectoryTreeState {
    fn default() -> Self {
        Self {
            open: false,
            panel: PanelId::Left,
            root: PathBuf::new(),
            entries: Vec::new(),
            selected: 0,
            viewport: ViewportState::default(),
            generation: 0,
            error: None,
        }
    }
}

impl DirectoryTreeState {
    pub fn open(&mut self, panel: PanelId, root: PathBuf) {
        let visible_rows = self.viewport.visible_rows;
        let generation = self.generation.wrapping_add(1).max(1);
        let name = root.display().to_string();

        *self = Self {
            open: true,
            panel,
            root: root.clone(),
            entries: vec![TreeEntry {
                is_hidden: false,
                path: root,
                name,
                depth: 0,
                is_directory: true,
                expanded: true,
                loading: true,
            }],
            viewport: ViewportState {
                visible_rows,
                ..ViewportState::default()
            },
            generation,
            ..Self::default()
        };
    }

    pub fn close(&mut self) {
        self.open = false;
        self.error = None;
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.viewport.visible_rows = rows;
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), 10);
    }

    pub fn apply_motion(&mut self, motion: SelectionMotion, count: Option<usize>) {
        apply_selection_motion(
            &mut self.selected,
            &mut self.viewport,
            self.entries.len(),
            motion,
            count,
            10,
        );
    }

    pub fn center_selection(&mut self) {
        self.viewport.center(self.selected, self.entries.len());
    }

    pub fn replace_children(&mut self, parent: &std::path::Path, mut children: Vec<TreeEntry>) {
        let Some(index) = self.entries.iter().position(|entry| entry.path == parent) else {
            return;
        };

        let depth = self.entries[index].depth;
        let descendants = self.descendant_range(index);
        self.entries.drain(descendants);
        self.entries[index].expanded = true;
        self.entries[index].loading = false;

        for child in &mut children {
            child.depth = depth + 1;
        }
        self.entries.splice(index + 1..index + 1, children);

        self.viewport
            .ensure_visible(self.selected, self.entries.len(), 10);
    }

    pub fn collapse_selected(&mut self) {
        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };

        if !entry.is_directory || !entry.expanded {
            self.select_parent(entry.depth);
            return;
        }

        let descendants = self.descendant_range(self.selected);
        self.entries.drain(descendants);
        self.entries[self.selected].expanded = false;
        self.entries[self.selected].loading = false;
    }

    pub fn collapse_all(&mut self) {
        self.entries.retain(|entry| entry.depth <= 1);
        for entry in &mut self.entries {
            entry.expanded = entry.depth == 0;
            entry.loading = false;
        }

        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.viewport.scroll = 0;
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), 10);
    }

    pub fn replace_all(&mut self, entries: Vec<TreeEntry>) {
        self.entries.truncate(1);
        self.entries[0].expanded = true;
        self.entries[0].loading = false;
        self.entries.extend(entries);

        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), 10);
    }

    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.entries.get(self.selected)
    }

    fn descendant_range(&self, index: usize) -> Range<usize> {
        let depth = self.entries[index].depth;
        let start = index + 1;
        let end = self.entries[start..]
            .iter()
            .position(|entry| entry.depth <= depth)
            .map_or(self.entries.len(), |offset| start + offset);

        start..end
    }

    fn select_parent(&mut self, depth: usize) {
        if depth == 0 {
            return;
        }

        if let Some(parent) = self.entries[..self.selected]
            .iter()
            .rposition(|entry| entry.depth < depth)
        {
            self.selected = parent;
            self.viewport
                .ensure_visible(self.selected, self.entries.len(), 10);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, depth: usize, directory: bool) -> TreeEntry {
        TreeEntry {
            is_hidden: false,
            path: path.into(),
            name: path.into(),
            depth,
            is_directory: directory,
            expanded: false,
            loading: false,
        }
    }

    #[test]
    fn children_are_inserted_below_their_parent_and_collapse_removes_descendants() {
        let mut tree = DirectoryTreeState::default();
        tree.open(PanelId::Left, "root".into());
        tree.replace_children(
            std::path::Path::new("root"),
            vec![entry("root/a", 0, true), entry("root/x", 0, false)],
        );
        tree.selected = 1;
        tree.replace_children(
            std::path::Path::new("root/a"),
            vec![entry("root/a/b", 0, false)],
        );
        assert_eq!(
            tree.entries
                .iter()
                .map(|item| item.depth)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 1]
        );
        tree.collapse_selected();
        assert_eq!(tree.entries.len(), 3);
        assert!(!tree.entries[1].expanded);
    }

    #[test]
    fn collapse_on_a_closed_node_moves_to_its_parent() {
        let mut tree = DirectoryTreeState {
            entries: vec![
                entry("root", 0, true),
                entry("root/a", 1, true),
                entry("root/a/b", 2, true),
            ],
            selected: 2,
            ..DirectoryTreeState::default()
        };
        tree.collapse_selected();
        assert_eq!(tree.selected, 1);
    }
}
