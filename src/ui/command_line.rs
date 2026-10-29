use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{AppMode, AppState, NotificationLevel};
use crate::dialogs::DialogDisplay;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let background = Style::default().bg(theme.command_background.into());
    frame.render_widget(Block::default().style(background), area);
    if let Some(prompt) = &state.rename_prompt {
        if prompt.display == DialogDisplay::CommandLine {
            super::rename::draw_input(
                frame,
                area,
                prompt,
                theme,
                background,
                "Rename | New name: ",
            );
        }
        return;
    }

    draw_content(frame, area, state, theme, background);
}

fn draw_content(
    frame: &mut Frame<'_>,
    inner: Rect,
    state: &AppState,
    theme: &Theme,
    background: Style,
) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let primary = primary_text(state);
    let command_line_dialog = state.mode == AppMode::Confirm
        && state.dialog_options.display == DialogDisplay::CommandLine;
    let spans = content_spans(&primary, state, theme, background, command_line_dialog);

    let primary_width = Line::from(primary.as_str()).width();
    let pending_width = if command_line_dialog {
        0
    } else {
        Line::from(state.pending_keys.as_str())
            .width()
            .min(usize::from(inner.width)) as u16
    };
    let available = inner
        .width
        .saturating_sub(pending_width)
        .saturating_sub(u16::from(pending_width > 0));
    let input_scroll = if command_line_dialog || state.mode == AppMode::Command {
        primary_width.saturating_sub(usize::from(available.saturating_sub(1)))
    } else {
        0
    };
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(background)
            .scroll((0, input_scroll.min(usize::from(u16::MAX)) as u16)),
        inner,
    );

    if pending_width > 0 {
        frame.render_widget(
            Paragraph::new(state.pending_keys.as_str()).style(background.fg(theme.text.into())),
            Rect::new(
                inner.right().saturating_sub(pending_width),
                inner.y,
                pending_width,
                1,
            ),
        );
    }

    if (state.mode == AppMode::Command
        || (state.mode == AppMode::Search && state.search.active)
        || command_line_dialog)
        && available > 0
    {
        let cursor = primary_width
            .saturating_sub(input_scroll)
            .min(usize::from(available.saturating_sub(1))) as u16;
        frame.set_cursor_position((inner.x + cursor, inner.y));
    }
}

fn content_spans<'a>(
    primary: &str,
    state: &'a AppState,
    theme: &Theme,
    background: Style,
    command_line_dialog: bool,
) -> Vec<Span<'a>> {
    let primary_color = if command_line_dialog {
        theme.dialog_color(state.dialog_kind)
    } else {
        theme.text
    };
    let mut spans = vec![Span::styled(
        primary.to_owned(),
        background.fg(primary_color.into()),
    )];
    if state.mode == AppMode::Search && state.search.no_match {
        spans.push(Span::styled(
            "  [no matches]",
            background.fg(theme.error.into()),
        ));
    }
    if let Some(notification) = &state.notification
        && !command_line_dialog
    {
        if !primary.is_empty() || spans.len() > 1 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            notification.message.as_str(),
            background.fg(if notification.level == NotificationLevel::Error {
                theme.error.into()
            } else {
                theme.inactive_title.into()
            }),
        ));
    }

    spans
}

fn primary_text(state: &AppState) -> String {
    match state.mode {
        AppMode::Command => state.command_line.clone(),
        AppMode::Confirm if state.dialog_options.display == DialogDisplay::CommandLine => {
            super::dialog::command_line_text(state)
        }
        AppMode::Confirm => String::new(),
        AppMode::Search => format!("/{}", state.search.query),
        AppMode::Insert => String::new(),
        AppMode::Visual => String::new(),
        AppMode::Normal if state.notification.is_some() => String::new(),
        AppMode::Normal => state
            .search
            .highlight_query
            .as_ref()
            .map(|query| format!("/{query}"))
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::dialogs::DialogDisplay;
    use crate::status_line::UiStyle;
    use crate::terminal::TerminalSize;
    use crate::workspace::WorkspaceOptions;

    use super::*;

    fn state() -> AppState {
        AppState::new(
            PathBuf::from("."),
            "test".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            WorkspaceOptions::default(),
        )
    }

    #[test]
    fn command_line_dialog_replaces_messages_and_shows_a_clipped_cursor() {
        use crate::app::{Action, Notification, update};
        use crate::workspace::DirectoryEntry;
        use ratatui::{Terminal, backend::TestBackend};
        let theme = Theme::emergency_monochrome();
        for style in [UiStyle::Framed, UiStyle::Borderless, UiStyle::Compact] {
            for width in [1, 30, 120] {
                let mut state = state();
                state.status_line_options.style = style;
                state.dialog_options.display = DialogDisplay::CommandLine;
                state.workspace.panels[0].entries = vec![DirectoryEntry {
                    name: "狼.txt".into(),
                    path: "folder/狼.txt".into(),
                    is_directory: false,
                    is_parent: false,
                    is_hidden: false,
                }];
                let area = Rect::new(2, 1, width, 1);
                let mut terminal = Terminal::new(TestBackend::new(width + 4, 3)).unwrap();
                state.notification = Some(Notification::info("Directory edits discarded"));
                terminal
                    .draw(|frame| draw(frame, area, &state, &theme))
                    .unwrap();
                update(&mut state, Action::RequestDelete);
                assert!(state.notification.is_none());
                // Asynchronous notifications and stale key displays must not join the prompt either.
                state.notification = Some(Notification::error("old error"));
                state.pending_keys = "gg".into();
                let prompt = primary_text(&state);
                terminal
                    .draw(|frame| draw(frame, area, &state, &theme))
                    .unwrap();
                let buffer = terminal.backend().buffer();
                let row: String = (area.x..area.right())
                    .map(|x| buffer[(x, area.y)].symbol())
                    .collect();
                assert!(!row.contains("discarded"));
                assert!(!row.contains("old error"));
                assert!(!row.contains("gg"));
                if width > 1 {
                    assert!(row.trim_end().ends_with("[D]elete   [C]ancel"));
                }
                let column = Line::from(prompt).width().min(usize::from(width - 1)) as u16;
                assert!(terminal.backend().cursor_visible());
                assert_eq!(
                    terminal.backend().cursor_position(),
                    (area.x + column, area.y).into()
                );
                assert_eq!(buffer[(area.x + column, area.y)].symbol(), " ");
                update(&mut state, Action::CancelConfirmation);
                state.pending_keys.clear();
                terminal
                    .draw(|frame| draw(frame, area, &state, &theme))
                    .unwrap();
                assert!(!terminal.backend().cursor_visible());
            }
        }
    }

    #[test]
    fn recalled_long_commands_keep_their_tail_and_cursor_visible() {
        use crate::app::{Action, update};
        use ratatui::{Terminal, backend::TestBackend};
        let mut state = state();
        state.command_history.record(":cd very-long-directory/狼");
        update(&mut state, Action::EnterCommandMode);
        update(&mut state, Action::PreviousCommandHistory);
        let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, frame.area(), &state, &Theme::emergency_monochrome()))
            .unwrap();
        let row: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(row.contains("狼"));
        assert!(terminal.backend().cursor_visible());
        assert_eq!(terminal.backend().cursor_position(), (11, 0).into());
        assert_eq!(terminal.backend().buffer()[(11, 0)].symbol(), " ");
    }

    #[test]
    fn float_dialog_does_not_claim_the_command_line_cursor() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut state = state();
        state.mode = AppMode::Confirm;
        state.dialog_options.display = DialogDisplay::Float;
        let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, frame.area(), &state, &Theme::emergency_monochrome()))
            .unwrap();
        assert!(!terminal.backend().cursor_visible());
    }

    #[test]
    fn confirmed_search_keeps_query_without_input_cursor() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut state = state();
        state.mode = AppMode::Search;
        state.search.query = "wolf".into();
        state.search.active = true;
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).unwrap();
        terminal
            .draw(|frame| draw(frame, frame.area(), &state, &Theme::emergency_monochrome()))
            .unwrap();
        assert!(terminal.backend().cursor_visible());

        state.search.active = false;
        terminal
            .draw(|frame| draw(frame, frame.area(), &state, &Theme::emergency_monochrome()))
            .unwrap();
        assert!(!terminal.backend().cursor_visible());
        assert_eq!(primary_text(&state), "/wolf");
    }

    #[test]
    fn command_and_search_prefixes_belong_only_to_the_command_line() {
        let mut state = state();
        assert_eq!(primary_text(&state), "");
        state.mode = AppMode::Command;
        state.command_line = ":rename wolf".into();
        assert_eq!(primary_text(&state), ":rename wolf");
        state.mode = AppMode::Search;
        state.search.query = "wolf".into();
        assert_eq!(primary_text(&state), "/wolf");
        state.mode = AppMode::Normal;
        state.search.highlight_query = Some("wolf".into());
        assert_eq!(primary_text(&state), "/wolf");
        state.notification = Some(crate::app::Notification::error("failure"));
        assert_eq!(primary_text(&state), "");
        state.directory_edit.active = true;
        state.directory_edit.awaiting_confirmation = true;
        state.mode = AppMode::Confirm;
        assert_eq!(primary_text(&state), "");
        state.dialog_options.display = DialogDisplay::CommandLine;
        assert!(primary_text(&state).contains("[Y]es   [N]o   [C]ancel"));
    }
}
