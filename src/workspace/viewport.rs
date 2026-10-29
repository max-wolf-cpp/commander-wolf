#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ViewportState {
    pub scroll: usize,
    pub visible_rows: usize,
}

impl ViewportState {
    pub fn ensure_visible(&mut self, selected: usize, row_count: usize, configured: usize) {
        if row_count == 0 || self.visible_rows == 0 {
            self.scroll = 0;
            return;
        }

        let selected = selected.min(row_count - 1);
        let scroll_offset = self.effective_scroll_offset(configured);
        let top_limit = self.scroll.saturating_add(scroll_offset);
        let bottom_margin = self.visible_rows - 1 - scroll_offset;
        let bottom_limit = self.scroll.saturating_add(bottom_margin);

        if selected < top_limit {
            self.scroll = selected.saturating_sub(scroll_offset);
        } else if selected > bottom_limit {
            self.scroll = selected.saturating_sub(bottom_margin);
        }

        self.clamp(row_count);
    }

    pub fn center(&mut self, selected: usize, row_count: usize) {
        if row_count == 0 || self.visible_rows == 0 {
            self.scroll = 0;
            return;
        }

        self.scroll = selected.saturating_sub(self.visible_rows / 2);
        self.clamp(row_count);
    }

    pub fn effective_scroll_offset(&self, configured: usize) -> usize {
        if self.visible_rows == 0 {
            0
        } else {
            configured.min((self.visible_rows - 1) / 2)
        }
    }

    pub fn top_index(&self, last: usize, configured: usize, count: usize) -> usize {
        let top = if self.visible_rows == 0 || self.scroll == 0 {
            0
        } else {
            let scroll_offset = self.effective_scroll_offset(configured);
            self.scroll.saturating_add(scroll_offset).min(last)
        };

        top.saturating_add(count.saturating_sub(1)).min(last)
    }

    pub fn middle_index(&self, last: usize) -> usize {
        if self.visible_rows == 0 {
            return last / 2;
        }

        let bottom = self.scroll.saturating_add(self.visible_rows - 1).min(last);

        self.scroll + (bottom - self.scroll) / 2
    }

    pub fn bottom_index(&self, last: usize, configured: usize, count: usize) -> usize {
        let visible_bottom = if self.visible_rows == 0 {
            last
        } else {
            self.scroll.saturating_add(self.visible_rows - 1).min(last)
        };
        let bottom = if visible_bottom == last {
            last
        } else {
            let scroll_offset = self.effective_scroll_offset(configured);
            visible_bottom.saturating_sub(scroll_offset)
        };

        bottom.saturating_sub(count.saturating_sub(1))
    }

    fn clamp(&mut self, row_count: usize) {
        let maximum = row_count.saturating_sub(self.visible_rows);
        self.scroll = self.scroll.min(maximum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_at_configured_boundaries_and_centers() {
        let mut viewport = ViewportState {
            scroll: 0,
            visible_rows: 23,
        };

        viewport.ensure_visible(12, 41, 10);
        assert_eq!(viewport.scroll, 0);
        viewport.ensure_visible(13, 41, 10);
        assert_eq!(viewport.scroll, 1);

        viewport.center(15, 41);
        assert_eq!(viewport.scroll, 4);
    }

    #[test]
    fn effective_offset_shrinks_for_small_viewports() {
        let viewport = ViewportState {
            scroll: 0,
            visible_rows: 5,
        };

        assert_eq!(viewport.effective_scroll_offset(10), 2);
    }
}
