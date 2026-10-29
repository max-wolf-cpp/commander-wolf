use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::dialogs::{DialogDisplay, DialogKind, RenamePrompt};
use crate::theme::Theme;

const ACTIONS: [(&str, &str); 3] = [
    ("Enter", "Rename"),
    ("Esc", "Cancel"),
    ("Ctrl+A", "Select all"),
];

pub(super) fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let Some(prompt) = state
        .rename_prompt
        .as_ref()
        .filter(|prompt| prompt.display == DialogDisplay::Float)
    else {
        return;
    };

    let area = dialog_area(frame.area());
    if area.is_empty() {
        return;
    }

    let kind = if prompt.error.is_some() {
        DialogKind::Error
    } else {
        DialogKind::Info
    };
    let mut colors = theme.dialog_colors(kind);
    colors.accent = theme
        .dialog_text_color(kind)
        .unwrap_or(theme.dialog_color(DialogKind::Info));
    let background = Style::default().bg(colors.background).fg(colors.text);
    let accent = background.fg(colors.accent);
    super::floating::clear(frame, area, colors.background);
    let inner = super::chrome::draw_titled_block(
        frame,
        area,
        Block::default()
            .borders(Borders::ALL)
            .style(background)
            .border_set(theme.border_set())
            .border_style(accent),
        "Rename",
        accent.add_modifier(Modifier::BOLD),
        background,
    )
    .inner(Margin::new(1, 0));
    let row = |offset| {
        Rect::new(
            inner.x,
            inner.y + offset,
            inner.width,
            u16::from(offset < inner.height),
        )
    };
    frame.render_widget(
        Paragraph::new(prompt.source.to_string_lossy()).style(background.fg(colors.secondary)),
        row(0),
    );
    frame.render_widget(Paragraph::new("New name").style(accent), row(2));
    draw_input(frame, row(3), prompt, theme, background, "");
    if let Some(error) = &prompt.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .style(background.fg(theme.dialog_text_color(kind).unwrap_or(theme.error))),
            row(4),
        );
    }

    frame.render_widget(
        Paragraph::new(actions_line(
            background.fg(theme.dialog_shortcut_color(kind, colors.accent)),
        ))
        .style(background),
        row(6),
    );
}

pub(super) fn draw_input(
    frame: &mut Frame<'_>,
    area: Rect,
    prompt: &RenamePrompt,
    theme: &Theme,
    background: Style,
    prefix: &str,
) {
    if area.is_empty() {
        return;
    }

    let mut input_style = background.fg(background.fg.unwrap_or(theme.text));
    if prompt.selected_all {
        input_style = input_style
            .fg(theme.selected_text.into())
            .bg(theme.selected.into());
    }

    let mut spans = vec![
        Span::styled(
            prefix,
            background.fg(theme.dialog_color(DialogKind::Info).into()),
        ),
        Span::styled(prompt.text.as_str(), input_style),
    ];
    if !prefix.is_empty()
        && let Some(error) = &prompt.error
    {
        spans.push(Span::styled(
            format!("  {error}"),
            background.fg(theme.error.into()),
        ));
    }
    if !prefix.is_empty() {
        for (key, label) in ACTIONS {
            spans.push(Span::raw(format!("   [{key}] {label}")));
        }
    }

    let cursor_width =
        Line::from(prefix).width() + Line::from(&prompt.text[..prompt.cursor]).width();
    let scroll = cursor_width.saturating_sub(usize::from(area.width.saturating_sub(1)));
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(background)
            .scroll((0, scroll.min(usize::from(u16::MAX)) as u16)),
        area,
    );
    frame.set_cursor_position((area.x + (cursor_width - scroll) as u16, area.y));
}

fn actions_line(accent: Style) -> Line<'static> {
    Line::from(
        ACTIONS
            .into_iter()
            .enumerate()
            .flat_map(|(index, (key, label))| {
                [
                    Span::raw(if index == 0 { "" } else { "   " }),
                    Span::styled(key, accent.add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {label}")),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

fn dialog_area(screen: Rect) -> Rect {
    let width = screen
        .width
        .saturating_sub(4)
        .clamp(1, 76)
        .min(screen.width);
    let height = screen.height.min(9);

    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::IconMode;
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn float_matches_theme_has_padding_and_clips_long_input() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 100,
                rows: 30,
            },
            WorkspaceOptions::default(),
        );
        state.rename_prompt = Some(RenamePrompt::new(
            "old.txt".into(),
            "旧名.txt".into(),
            DialogDisplay::Float,
        ));
        let theme = Theme::emergency_monochrome();
        for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
            state.workspace.icon_mode = mode;
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
            terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
            let area = dialog_area(Rect::new(0, 0, 100, 30));
            let buffer = terminal.backend().buffer();
            let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
            assert!(text.contains("Rename"));
            assert!(text.contains("Enter Rename"));
            assert_eq!(buffer[(area.x + 1, area.y + 4)].symbol(), " ");
            assert_eq!(buffer[(area.x + 2, area.y + 4)].symbol(), "旧");
            assert_eq!(
                buffer[(area.x, area.y)].fg,
                theme.dialog_color(DialogKind::Info).into()
            );
            assert_eq!(
                buffer[(area.x + 1, area.y + 1)].bg,
                theme.float_background.into()
            );
            assert_eq!(
                terminal.backend().cursor_position(),
                (area.x + 10, area.y + 4).into()
            );
        }
        state.rename_prompt = Some(RenamePrompt::new(
            "old".into(),
            "猫".repeat(200),
            DialogDisplay::Float,
        ));
        for (width, height) in [(20, 12), (5, 5), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
            if height == 12 {
                let cursor = terminal.backend().cursor_position();
                assert!(cursor.x < width && cursor.y < height);
            }
        }
    }

    #[test]
    fn command_line_input_replaces_previous_message_and_shows_cursor() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 30,
                rows: 1,
            },
            WorkspaceOptions::default(),
        );
        state.rename_prompt = Some(RenamePrompt::new(
            "old".into(),
            "new name.txt".into(),
            DialogDisplay::CommandLine,
        ));
        state.notification = Some(crate::app::Notification::info("Previous message"));
        let mut terminal = Terminal::new(TestBackend::new(120, 1)).unwrap();
        terminal
            .draw(|frame| {
                super::super::command_line::draw(
                    frame,
                    Rect::new(0, 0, 120, 1),
                    &state,
                    &Theme::emergency_monochrome(),
                )
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.starts_with("Rename | New name: new name.txt"));
        assert!(text.contains("[Enter] Rename   [Esc] Cancel   [Ctrl+A] Select all"));
        assert!(!text.contains("Previous"));
        assert_eq!(terminal.backend().cursor_position(), (31, 0).into());
    }
}
