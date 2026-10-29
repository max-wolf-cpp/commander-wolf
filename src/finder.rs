use std::path::PathBuf;

use crate::workspace::{PanelId, SelectionMotion, ViewportState, apply_selection_motion};

mod glob;
mod result;

pub use glob::GlobMatcher;
pub use result::{FinderEntryKind, FinderResult, FinderWorkerMessage};

pub const MAX_CONFIGURED_RESULTS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinderOptions {
    pub show_hidden: bool,
    pub scroll_offset: usize,
    pub max_results: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinderSearchRequest {
    pub generation: u64,
    pub root: PathBuf,
    pub matcher: GlobMatcher,
    pub options: FinderOptions,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FinderMode {
    #[default]
    Closed,
    Insert,
    Searching,
    Normal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinderState {
    pub mode: FinderMode,
    pub panel: PanelId,
    pub root: PathBuf,
    pub pattern: String,
    pub results: Vec<FinderResult>,
    pub selected: usize,
    pub viewport: ViewportState,
    pub generation: u64,
    pub skipped: usize,
    pub truncated: bool,
    pub error: Option<String>,
}

impl Default for FinderOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            scroll_offset: 10,
            max_results: 10_000,
        }
    }
}

impl Default for FinderState {
    fn default() -> Self {
        Self {
            mode: FinderMode::Closed,
            panel: PanelId::Left,
            root: PathBuf::new(),
            pattern: String::new(),
            results: Vec::new(),
            selected: 0,
            viewport: ViewportState::default(),
            generation: 0,
            skipped: 0,
            truncated: false,
            error: None,
        }
    }
}

impl FinderState {
    pub fn open(&mut self, panel: PanelId, root: PathBuf) {
        let visible_rows = self.viewport.visible_rows;

        *self = Self {
            mode: FinderMode::Insert,
            panel,
            root,
            viewport: ViewportState {
                visible_rows,
                ..ViewportState::default()
            },
            ..Self::default()
        };
    }

    pub fn close(&mut self) {
        self.mode = FinderMode::Closed;
        self.error = None;
    }

    pub fn finalize(&mut self) {
        let selected_path = self
            .selected_result()
            .map(|result| result.absolute_path.clone());

        self.results
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        self.selected = selected_path
            .and_then(|path| {
                self.results
                    .iter()
                    .position(|result| result.absolute_path == path)
            })
            .unwrap_or(0);

        self.viewport
            .ensure_visible(self.selected, self.results.len(), 0);
        self.mode = FinderMode::Normal;
    }

    pub fn apply_motion(
        &mut self,
        motion: SelectionMotion,
        count: Option<usize>,
        scroll_offset: usize,
    ) {
        apply_selection_motion(
            &mut self.selected,
            &mut self.viewport,
            self.results.len(),
            motion,
            count,
            scroll_offset,
        );
    }

    pub fn center_selection(&mut self) {
        self.viewport.center(self.selected, self.results.len());
    }

    pub fn set_visible_rows(&mut self, rows: usize, scroll_offset: usize) {
        self.viewport.visible_rows = rows;
        self.viewport
            .ensure_visible(self.selected, self.results.len(), scroll_offset);
    }

    pub fn is_open(&self) -> bool {
        self.mode != FinderMode::Closed
    }

    pub fn selected_result(&self) -> Option<&FinderResult> {
        self.results.get(self.selected)
    }
}
