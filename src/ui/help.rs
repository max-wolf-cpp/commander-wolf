use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::AppState;
use crate::help::HelpState;
use crate::theme::Theme;

pub fn visible_rows(screen: Rect) -> usize {
    usize::from(modal_area(screen).height.saturating_sub(4))
}

pub(super) fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let Some(help) = &state.help else {
        return;
    };

    let area = modal_area(frame.area());
    if area.width < 12 || area.height < 5 {
        return;
    }

    let background = Style::default().bg(theme.float_background);
    let text = background.fg(theme.text);
    let accent = background.fg(theme.active_border);
    super::floating::clear(frame, area, theme.float_background);
    let inner = super::chrome::draw_titled_block(
        frame,
        area,
        Block::default()
            .borders(Borders::ALL)
            .style(background)
            .border_set(theme.border_set())
            .border_style(accent),
        "Commander Wolf 0.1.0",
        accent,
        background,
    )
    .inner(Margin::new(1, 0));
    if inner.width == 0 || inner.height < 2 {
        return;
    }

    draw_bindings(frame, inner, help, visible_rows(frame.area()), text, accent);
}

fn modal_area(screen: Rect) -> Rect {
    let width = screen.width.saturating_sub(4).min(110);
    let height = screen.height.saturating_sub(4).min(36);
    Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn draw_bindings(
    frame: &mut Frame<'_>,
    inner: Rect,
    help: &HelpState,
    visible: usize,
    text: Style,
    accent: Style,
) {
    let start = help.scroll.min(help.entries.len().saturating_sub(visible));
    let end = start.saturating_add(visible).min(help.entries.len());
    let rows = help.entries[start..end].iter().map(|entry| {
        Row::new([
            Cell::from(entry.context.label()),
            Cell::from(entry.keys.as_str()),
            Cell::from(entry.description.as_str()),
        ])
    });
    let mode_width = if inner.width >= 55 { 15 } else { 10 };
    let key_width = if inner.width >= 55 { 18 } else { 12 };
    let table = Table::new(
        rows,
        [
            Constraint::Length(mode_width),
            Constraint::Length(key_width),
            Constraint::Min(1),
        ],
    )
    .header(Row::new(["Mode", "Keys", "Description"]).style(accent))
    .column_spacing(1)
    .style(text);
    frame.render_widget(
        table,
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1),
    );

    let hint = format!(
        "{}-{} / {}  |  Up/Down/PgUp/PgDn: scroll  Esc/q: close",
        if help.entries.is_empty() {
            0
        } else {
            start + 1
        },
        end,
        help.entries.len(),
    );
    frame.render_widget(
        Paragraph::new(hint).style(accent),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::help::{HelpEntry, HelpState};
    use crate::input::InputContext;
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn help_float_shows_version_and_custom_binding() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 90,
                rows: 30,
            },
            WorkspaceOptions::default(),
        );
        state.help = Some(HelpState {
            entries: vec![HelpEntry {
                context: InputContext::Normal,
                keys: "y".into(),
                description: "Copy selection short alias".into(),
            }],
            scroll: 0,
        });
        let mut terminal = Terminal::new(TestBackend::new(90, 30)).unwrap();
        terminal
            .draw(|frame| draw(frame, &state, &Theme::emergency_monochrome()))
            .unwrap();
        let displayed = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(displayed.contains("Commander Wolf 0.1.0"));
        assert!(displayed.contains("Copy selection short alias"));
    }

    #[test]
    fn help_float_is_safe_on_small_terminals() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 20,
                rows: 10,
            },
            WorkspaceOptions::default(),
        );
        state.help = Some(HelpState {
            entries: vec![HelpEntry {
                context: InputContext::Normal,
                keys: "<Space>ff".into(),
                description: "Open file finder".into(),
            }],
            scroll: 0,
        });
        let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();
        terminal
            .draw(|frame| draw(frame, &state, &Theme::emergency_monochrome()))
            .unwrap();
    }
}
