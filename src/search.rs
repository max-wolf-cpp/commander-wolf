mod matching;

use crate::workspace::{DirectoryEntry, PanelId};

use matching::find_match;
pub use matching::{SearchDirection, match_ranges, repeat_match};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchState {
    pub active: bool,
    pub confirmed_active: bool,
    pub panel: Option<PanelId>,
    pub query: String,
    pub original_selection: usize,
    pub last_confirmed_query: Option<String>,
    pub highlight_panel: Option<PanelId>,
    pub highlight_query: Option<String>,
    pub current_match: Option<usize>,
    pub no_match: bool,
    previous_highlight_panel: Option<PanelId>,
    previous_highlight_query: Option<String>,
    previous_current_match: Option<usize>,
    previous_confirmed_active: bool,
}

impl SearchState {
    pub fn begin(&mut self, panel: PanelId, selection: usize) {
        self.previous_highlight_panel = self.highlight_panel;
        self.previous_highlight_query = self.highlight_query.clone();
        self.previous_current_match = self.current_match;
        self.previous_confirmed_active = self.confirmed_active;

        self.active = true;
        self.panel = Some(panel);
        self.query.clear();
        self.original_selection = selection;

        self.highlight_panel = None;
        self.highlight_query = None;
        self.current_match = None;
        self.no_match = false;
    }

    pub fn submit(&mut self) {
        self.active = false;
        if self.query.is_empty() {
            self.restore_previous_highlight();
            self.confirmed_active = self.previous_confirmed_active;
        } else {
            self.last_confirmed_query = Some(self.query.clone());
            self.confirmed_active = true;
        }

        self.panel = None;
        self.clear_saved_highlight();
    }

    pub fn cancel(&mut self) -> usize {
        self.active = false;
        self.panel = None;
        self.query.clear();
        self.no_match = false;

        self.restore_previous_highlight();
        self.confirmed_active = self.previous_confirmed_active;
        self.previous_confirmed_active = false;

        self.original_selection
    }

    pub fn dismiss_confirmed(&mut self) {
        self.active = false;
        self.confirmed_active = false;
        self.panel = None;
        self.query.clear();
        self.highlight_panel = None;
        self.highlight_query = None;
        self.current_match = None;
        self.no_match = false;

        self.clear_saved_highlight();
    }

    pub fn update_draft(&mut self, entries: &[DirectoryEntry]) -> usize {
        let Some(panel) = self.panel else {
            return self.original_selection;
        };

        if self.query.is_empty() {
            self.highlight_panel = None;
            self.highlight_query = None;
            self.current_match = None;
            self.no_match = false;
            return self.original_selection;
        }

        let found = find_match(
            entries,
            &self.query,
            self.original_selection,
            SearchDirection::Forward,
            1,
            true,
        );

        self.highlight_panel = Some(panel);
        self.highlight_query = Some(self.query.clone());
        self.current_match = found;
        self.no_match = found.is_none();

        found.unwrap_or(self.original_selection)
    }

    pub fn clear_visuals_for(&mut self, panel: PanelId) {
        if self.highlight_panel == Some(panel) {
            self.highlight_panel = None;
            self.highlight_query = None;
            self.current_match = None;
            self.no_match = false;
            self.confirmed_active = false;
        }
    }

    pub fn highlighted_query(&self, panel: PanelId) -> Option<&str> {
        (self.highlight_panel == Some(panel))
            .then_some(self.highlight_query.as_deref())
            .flatten()
    }

    fn clear_saved_highlight(&mut self) {
        self.previous_highlight_panel = None;
        self.previous_highlight_query = None;
        self.previous_current_match = None;
        self.previous_confirmed_active = false;
    }

    fn restore_previous_highlight(&mut self) {
        self.highlight_panel = self.previous_highlight_panel;
        self.highlight_query = self.previous_highlight_query.take();
        self.current_match = self.previous_current_match;
    }
}

#[cfg(test)]
mod tests;
