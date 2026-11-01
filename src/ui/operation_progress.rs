use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::dialogs::DialogKind;
use crate::file_operations::FileOperationKind;
use crate::file_operations::progress::OperationProgress;
use crate::theme::{DialogColors, Theme};

pub(super) fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let Some(progress) = &state.operation_progress else {
        return;
    };

    let area = modal_area(frame.area());
    if area.is_empty() {
        return;
    }

    let mut colors = theme.dialog_colors(DialogKind::Info);
    colors.accent = theme.dialog_info_text.unwrap_or(theme.mode_command);
    let background = Style::default().bg(colors.background);
    let accent = background.fg(colors.accent);
    let title = match progress.kind {
        FileOperationKind::Copy => "Copy",
        FileOperationKind::Move => "Move",
        FileOperationKind::Delete => "Delete",
        FileOperationKind::CreateArchive => "Create ZIP",
        FileOperationKind::ExtractArchive => "Extract archive",
        _ => "File operation",
    };
    super::floating::clear(frame, area, colors.background);
    let inner = super::chrome::draw_titled_block(
        frame,
        area,
        Block::default()
            .borders(Borders::ALL)
            .style(background)
            .border_set(theme.border_set())
            .border_style(accent),
        title,
        accent.add_modifier(Modifier::BOLD),
        background,
    )
    .inner(Margin::new(1, 0));

    draw_details(frame, inner, progress, colors);
    draw_progress(frame, inner, progress, theme, colors);
    draw_cancel_hint(frame, inner, progress, theme, colors);
}

fn modal_area(screen: Rect) -> Rect {
    let width = screen.width.min(80);
    let height = screen.height.min(12);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

fn percent(done: u64, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        (u128::from(done.min(total)) * 100 / u128::from(total)) as u64
    }
}

fn content_line(inner: Rect, row: u16) -> Rect {
    Rect::new(
        inner.x,
        inner.y.saturating_add(row),
        inner.width,
        u16::from(row < inner.height),
    )
}

fn draw_details(
    frame: &mut Frame<'_>,
    inner: Rect,
    progress: &OperationProgress,
    colors: DialogColors,
) {
    let background = Style::default().bg(colors.background);
    let accent = background.fg(colors.accent);
    let text = background.fg(colors.text);
    frame.render_widget(
        Paragraph::new(if progress.cancelling {
            "Cancelling safely..."
        } else {
            &progress.phase
        })
        .style(accent),
        content_line(inner, 0),
    );
    frame.render_widget(
        Paragraph::new(
            progress
                .path
                .file_name()
                .unwrap_or(progress.path.as_os_str())
                .to_string_lossy(),
        )
        .style(text),
        content_line(inner, 1),
    );
    frame.render_widget(
        Paragraph::new(progress.path.to_string_lossy()).style(background.fg(colors.secondary)),
        content_line(inner, 2),
    );
}

fn draw_progress(
    frame: &mut Frame<'_>,
    inner: Rect,
    progress: &OperationProgress,
    theme: &Theme,
    colors: DialogColors,
) {
    let text = Style::default().bg(colors.background).fg(colors.text);
    frame.render_widget(
        Paragraph::new(format!(
            "Current file: {}%",
            percent(progress.file_done, progress.file_total)
        ))
        .style(text),
        content_line(inner, 3),
    );
    draw_bar(
        frame,
        content_line(inner, 4),
        progress.file_done,
        progress.file_total,
        theme,
    );
    frame.render_widget(
        Paragraph::new(format!(
            "Overall: {}%",
            percent(progress.total_done, progress.total)
        ))
        .style(text),
        content_line(inner, 6),
    );
    draw_bar(
        frame,
        content_line(inner, 7),
        progress.total_done,
        progress.total,
        theme,
    );
}

fn draw_cancel_hint(
    frame: &mut Frame<'_>,
    inner: Rect,
    progress: &OperationProgress,
    theme: &Theme,
    colors: DialogColors,
) {
    if inner.height == 0 {
        return;
    }

    let background = Style::default().bg(colors.background);
    let shortcut = background.fg(theme.dialog_shortcut_color(DialogKind::Info, colors.accent));
    let text = background.fg(colors.text);
    let label = if progress.cancelling {
        Line::raw("Waiting for the current filesystem step to finish")
    } else {
        Line::from(vec![
            Span::styled("C", shortcut.add_modifier(Modifier::BOLD)),
            Span::raw("ancel  (Esc / Ctrl+C)"),
        ])
    };
    frame.render_widget(
        Paragraph::new(label).style(text),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

fn draw_bar(frame: &mut Frame<'_>, area: Rect, done: u64, total: u64, theme: &Theme) {
    if area.is_empty() {
        return;
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(theme.selected)),
        area,
    );
    let filled = if total == 0 {
        0
    } else {
        (u128::from(done.min(total)) * u128::from(area.width) / u128::from(total)) as u16
    };
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.mode_command)),
        Rect::new(area.x, area.y, filled, area.height),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_operations::progress::OperationProgress;
    use crate::icons::IconMode;
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn mandatory_progress_uses_theme_and_remains_bounded() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 100,
                rows: 30,
            },
            WorkspaceOptions::default(),
        );
        let mut progress = OperationProgress::new(FileOperationKind::Copy);
        progress.file_done = 25;
        progress.file_total = 100;
        progress.total_done = 50;
        progress.total = 100;
        state.operation_progress = Some(progress);
        let theme = Theme::emergency_monochrome();
        for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
            state.workspace.icon_mode = mode;
            for (width, height) in [(100, 30), (20, 8), (1, 1)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                if width == 100 {
                    let area = modal_area(Rect::new(0, 0, width, height));
                    let buffer = terminal.backend().buffer();
                    let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
                    assert!(text.contains("Current file: 25%"));
                    assert!(text.contains("Overall: 50%"));
                    assert!(text.contains("Cancel"));
                    assert_eq!(buffer[(area.x, area.y)].fg, theme.mode_command);
                }
            }
        }
    }
}
