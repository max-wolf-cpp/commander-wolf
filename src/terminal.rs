#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub fn area(self) -> ratatui::layout::Rect {
        ratatui::layout::Rect::new(0, 0, self.columns, self.rows)
    }
}
