mod directory;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{SelectionMotion, ViewportState, apply_selection_motion};

pub use directory::{DirectoryEntry, DirectorySnapshot};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PanelState {
    pub revision: u64,
    pub hide_hidden: bool,
    pub all_entries: Vec<DirectoryEntry>,
    pub directory: PathBuf,
    pub entries: Vec<DirectoryEntry>,
    pub selected: usize,
    pub viewport: ViewportState,
    pub marked: BTreeSet<PathBuf>,
}

impl PanelState {
    pub fn apply_motion(
        &mut self,
        motion: SelectionMotion,
        count: Option<usize>,
        configured_scroll_offset: usize,
    ) {
        apply_selection_motion(
            &mut self.selected,
            &mut self.viewport,
            self.entries.len(),
            motion,
            count,
            configured_scroll_offset,
        );
    }

    pub fn center_selection(&mut self) {
        self.viewport.center(self.selected, self.entries.len());
    }

    pub fn set_visible_rows(&mut self, rows: usize, configured_scroll_offset: usize) {
        self.viewport.visible_rows = rows;
        self.viewport
            .ensure_visible(self.selected, self.entries.len(), configured_scroll_offset);
    }

    pub fn toggle_mark(&mut self, path: &Path) -> bool {
        if self.marked.remove(path) {
            false
        } else {
            self.marked.insert(path.to_path_buf());
            true
        }
    }

    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    pub fn selected_entry(&self) -> Option<&DirectoryEntry> {
        self.entries.get(self.selected)
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_entry().map(|entry| entry.path.as_path())
    }

    pub fn is_marked(&self, path: &Path) -> bool {
        self.marked.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(count: usize) -> PanelState {
        let mut panel = PanelState {
            entries: (0..count)
                .map(|index| DirectoryEntry {
                    path: PathBuf::from(index.to_string()),
                    name: index.to_string(),
                    is_directory: false,
                    is_parent: false,
                    is_hidden: false,
                })
                .collect(),
            ..PanelState::default()
        };
        panel.set_visible_rows(10, 3);

        panel
    }

    #[test]
    fn hidden_filter_preserves_selection_prunes_marks_and_survives_refresh() {
        let mut panel = panel(4);
        panel.entries[0].is_parent = true;
        panel.entries[1].is_hidden = true;
        panel.entries[2].is_hidden = true;
        panel.entries[2].is_directory = true;
        panel.selected = 3;
        panel.toggle_mark(Path::new("1"));
        panel.toggle_mark(Path::new("3"));
        panel.set_show_hidden(false, 0);
        assert_eq!(panel.entries.len(), 2);
        assert_eq!(panel.selected_path(), Some(Path::new("3")));
        assert!(!panel.is_marked(Path::new("1")));
        assert!(panel.is_marked(Path::new("3")));

        panel.refresh_directory(
            DirectorySnapshot {
                directory: "root".into(),
                entries: panel.all_entries.clone(),
                selected: 3,
            },
            0,
        );
        assert_eq!(panel.entries.len(), 2);
        assert_eq!(panel.selected_path(), Some(Path::new("3")));

        panel.set_show_hidden(true, 0);
        assert_eq!(panel.entries.len(), 4);
        assert_eq!(panel.selected_path(), Some(Path::new("3")));

        panel.selected = 2;
        panel.set_show_hidden(false, 0);
        assert!(panel.selected_entry().is_some_and(|entry| !entry.is_hidden));
    }

    #[test]
    fn vim_motions_clamp_and_use_the_actual_viewport() {
        let mut panel = panel(30);

        panel.apply_motion(SelectionMotion::Next, Some(15), 3);
        assert_eq!(panel.selected, 15);
        assert_eq!(panel.viewport.scroll, 9);

        panel.apply_motion(SelectionMotion::ViewportTop, None, 3);
        assert_eq!(panel.selected, 12);
        panel.apply_motion(SelectionMotion::ViewportMiddle, None, 3);
        assert_eq!(panel.selected, 13);
        panel.apply_motion(SelectionMotion::ViewportBottom, None, 3);
        assert_eq!(panel.selected, 15);

        panel.center_selection();
        assert_eq!(panel.viewport.scroll, 10);

        panel.apply_motion(SelectionMotion::Previous, Some(100), 3);
        assert_eq!(panel.selected, 0);
        panel.apply_motion(SelectionMotion::Next, Some(100), 3);
        assert_eq!(panel.selected, 29);
    }

    #[test]
    fn empty_panel_motions_are_safe() {
        let mut panel = panel(0);

        panel.apply_motion(SelectionMotion::Last, Some(1), 10);
        assert_eq!(panel.selected, 0);
        assert_eq!(panel.viewport.scroll, 0);
    }

    #[test]
    fn refresh_preserves_viewport_when_the_selection_still_fits() {
        let mut panel = panel(30);
        panel.selected = 15;
        panel.viewport.scroll = 10;
        let snapshot = DirectorySnapshot {
            directory: PathBuf::from("refreshed"),
            entries: panel.entries.clone(),
            selected: 15,
        };
        panel.refresh_directory(snapshot, 3);
        assert_eq!(panel.viewport.scroll, 10);
        assert_eq!(panel.selected, 15);
    }

    #[test]
    fn marks_toggle_and_follow_only_entries_that_survive_refresh() {
        let mut panel = panel(3);
        assert!(panel.toggle_mark(Path::new("0")));
        assert!(panel.toggle_mark(Path::new("1")));
        assert!(!panel.toggle_mark(Path::new("0")));
        assert!(!panel.is_marked(Path::new("0")));
        assert!(panel.is_marked(Path::new("1")));

        panel.refresh_directory(
            DirectorySnapshot {
                directory: PathBuf::from("refreshed"),
                entries: vec![panel.entries[2].clone()],
                selected: 0,
            },
            3,
        );
        assert!(panel.marked.is_empty());

        panel.toggle_mark(Path::new("2"));
        panel.replace_directory(PathBuf::from("other"), Vec::new(), 0, 3);
        assert!(panel.marked.is_empty());
    }
}
