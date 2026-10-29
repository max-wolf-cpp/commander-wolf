use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::icons::IconMode;
use crate::shell::ShellOutput;
use crate::theme::Theme;

pub(super) fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let Some(output) = &state.shell_output else {
        return;
    };

    let area = popup_area(frame.area(), output.lines.len());
    if area.width < 5 || area.height < 3 {
        return;
    }

    let background = Style::default().bg(theme.float_background.into());
    let text = background.fg(theme.text.into());
    let accent = background.fg(theme.mode_command.into());
    super::floating::clear(frame, area, theme.float_background);
    let inner = super::chrome::draw_titled_block(
        frame,
        area,
        Block::default()
            .borders(Borders::ALL)
            .style(background)
            .border_set(theme.border_set())
            .border_style(accent),
        "Command output",
        accent,
        background,
    )
    .inner(Margin::new(1, 0));
    if inner.is_empty() {
        return;
    }

    frame.render_widget(
        Paragraph::new(format!(":!{}", output.command)).style(text),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let output_area = Rect::new(
        inner.x,
        inner.y.saturating_add(2),
        inner.width,
        inner.height.saturating_sub(3),
    );
    let first = draw_output(frame, output_area, output, text);
    draw_scrollbar(
        frame,
        Rect::new(area.right() - 1, output_area.y, 1, output_area.height),
        first,
        output.lines.len(),
        state.workspace.icon_mode == IconMode::None,
        accent,
    );

    if inner.height > 1 {
        frame.render_widget(
            Paragraph::new(output_hint(output)).style(if output.failed {
                background.fg(theme.error.into())
            } else {
                accent
            }),
            Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
        );
    }
    frame.set_cursor_position((inner.x, inner.bottom() - 1));
}

fn popup_area(screen: Rect, lines: usize) -> Rect {
    let height = lines.saturating_add(5).min(usize::from(screen.height / 2)) as u16;
    Rect::new(
        screen.x,
        screen.bottom().saturating_sub(height),
        screen.width,
        height,
    )
}

fn draw_output(frame: &mut Frame<'_>, area: Rect, output: &ShellOutput, text: Style) -> usize {
    let rows = area.height;
    let last_start = output.lines.len().saturating_sub(usize::from(rows));
    let first = last_start.saturating_sub(output.scroll);

    for (row, line) in output
        .lines
        .iter()
        .skip(first)
        .take(usize::from(rows))
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(line.as_str()).style(text),
            Rect::new(area.x, area.y + row as u16, area.width, 1),
        );
    }

    first
}

fn draw_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    first: usize,
    lines: usize,
    ascii: bool,
    accent: Style,
) {
    let rows = area.height;
    if lines <= usize::from(rows) || rows == 0 {
        return;
    }

    let last_start = lines.saturating_sub(usize::from(rows));
    let thumb = first * usize::from(rows - 1) / last_start.max(1);
    for row in 0..rows {
        let glyph = match (ascii, usize::from(row) == thumb) {
            (true, true) => "#",
            (true, false) => "|",
            (false, true) => "┃",
            (false, false) => "│",
        };
        frame.render_widget(
            Paragraph::new(glyph).style(accent),
            Rect::new(area.x, area.y + row, 1, 1),
        );
    }
}

fn output_hint(output: &ShellOutput) -> String {
    let status = output.finished.as_deref().unwrap_or("Running");
    let controls = if output.finished.is_some() {
        "Enter/Esc: close  : command"
    } else {
        "Ctrl+C: stop"
    };
    format!(
        "{status}{} | {controls} | Up/Down/PgUp/PgDn: scroll",
        if output.truncated {
            " (output truncated)"
        } else {
            ""
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::ShellOutput;
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn output_is_half_height_scrollable_and_uses_theme_colors() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            WorkspaceOptions::default(),
        );
        let mut output = ShellOutput::new("echo hello".into());
        for i in 0..50 {
            output.append(&format!("line{i}\n"));
        }
        state.shell_output = Some(output);
        let theme = Theme::emergency_monochrome();
        for (mode, width, height) in [
            crate::icons::IconMode::NerdFont,
            crate::icons::IconMode::Unicode,
            crate::icons::IconMode::None,
        ]
        .into_iter()
        .flat_map(|mode| {
            [(80, 24), (30, 12), (10, 6), (1, 1)].map(|(width, height)| (mode, width, height))
        }) {
            state.workspace.icon_mode = mode;
            let area = popup_area(Rect::new(0, 0, width, height), 50);
            assert!(area.height <= height / 2);
            assert_eq!(area.bottom(), height);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
            if width == 80 {
                let buffer = terminal.backend().buffer();
                let rendered: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
                assert!(
                    !rendered
                        .chars()
                        .any(|ch| ('\u{e000}'..='\u{f8ff}').contains(&ch))
                );
                if mode == crate::icons::IconMode::None {
                    assert_eq!(buffer[(area.x, area.y)].symbol(), "┌");
                }
                assert_eq!(buffer[(0, area.y)].fg, theme.mode_command.into());
                assert_eq!(buffer[(1, area.y + 1)].bg, theme.float_background.into());
                state.shell_output.as_mut().unwrap().scroll = usize::MAX;
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                let buffer = terminal.backend().buffer();
                let row: String = (0..width)
                    .map(|x| buffer[(x, area.y + 3)].symbol())
                    .collect();
                assert!(row.contains("line0"));
            }
        }
    }
}
