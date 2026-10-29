use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Clear};

use ratatui::style::Color;

pub(super) fn clear(frame: &mut Frame<'_>, area: Rect, background: Color) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(background.into())),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Rgb;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    #[test]
    fn clearing_a_float_never_touches_cells_outside_its_frame() {
        let area = Rect::new(2, 1, 4, 3);
        let background = Rgb {
            red: 1,
            green: 2,
            blue: 3,
        };
        let mut terminal = Terminal::new(TestBackend::new(8, 5)).unwrap();

        terminal
            .draw(|frame| clear(frame, area, background.into()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(area.x, area.y)].bg, background.into());
        assert_eq!(
            buffer[(area.right() - 1, area.bottom() - 1)].bg,
            background.into()
        );
        assert_eq!(buffer[(area.x - 1, area.y)].bg, Color::Reset);
        assert_eq!(buffer[(area.right(), area.y)].bg, Color::Reset);
        assert_eq!(buffer[(area.x, area.y - 1)].bg, Color::Reset);
        assert_eq!(buffer[(area.x, area.bottom())].bg, Color::Reset);
    }
}
