use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use crate::app::{AppMode, AppState};
use crate::dialogs::DialogDisplay;
use crate::finder::FinderMode;
use crate::status_line::{ModePosition, StatusMode, UiStyle};
use crate::theme::Theme;

struct StatusLayout {
    mode: Rect,
    separator: Rect,
    information: Rect,
}

pub fn draw(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    match state.status_line_options.style {
        UiStyle::Framed => draw_plain(frame, area, state, theme),
        UiStyle::Borderless | UiStyle::Compact => draw_compact(frame, area, state, theme),
    }
}

pub(super) fn current_mode(state: &AppState) -> StatusMode {
    if state.finder.is_open() {
        return match state.finder.mode {
            FinderMode::Insert => StatusMode::Insert,
            FinderMode::Searching => StatusMode::Search,
            FinderMode::Normal | FinderMode::Closed => StatusMode::Normal,
        };
    }

    match state.mode {
        AppMode::Normal => StatusMode::Normal,
        AppMode::Insert => StatusMode::Insert,
        AppMode::Command => StatusMode::Command,
        AppMode::Confirm if state.dialog_options.display == DialogDisplay::CommandLine => {
            StatusMode::Command
        }
        AppMode::Confirm => StatusMode::Normal,
        AppMode::Search => StatusMode::Search,
        AppMode::Visual => StatusMode::Visual,
    }
}

fn draw_plain(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let background = Style::default().bg(theme.status_background);
    frame.render_widget(Block::default().style(background), area);
    if area.is_empty() {
        return;
    }

    let mode = current_mode(state);
    let mode_width = (mode.label().len() as u16).min(area.width);
    let layout = plain_layout(area, mode_width, state.status_line_options.mode_position);
    let mode_style = bold_if(background.fg(theme.mode_color(mode)), theme.status_bold);
    frame.render_widget(Paragraph::new(mode.label()).style(mode_style), layout.mode);

    let information_style = background.fg(theme.inactive_title);
    if layout.separator.width > 0 {
        let separator = if state.workspace.icon_mode == crate::icons::IconMode::None {
            " | "
        } else {
            " · "
        };
        frame.render_widget(
            Paragraph::new(separator).style(information_style),
            layout.separator,
        );
    }

    if layout.information.width > 0 {
        frame.render_widget(
            Paragraph::new(information(state)).style(information_style),
            layout.information,
        );
    }
}

fn draw_compact(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    if area.is_empty() {
        return;
    }

    let background = Style::default().bg(theme.compact_status_background);
    frame.render_widget(Block::default().style(background), area);
    let mode = current_mode(state);
    let mode_background = theme.compact_mode_background_color(mode);
    let label = format!(" {} ", mode.label());
    let label_width = Line::from(label.as_str()).width() as u16;
    let glyph = if theme.decorations {
        match state.status_line_options.mode_position {
            ModePosition::Left => theme.compact_mode_separator_left.as_str(),
            ModePosition::Right => theme.compact_mode_separator_right.as_str(),
        }
    } else {
        ""
    };
    let layout = compact_layout(
        area,
        label_width,
        glyph,
        state.status_line_options.mode_position,
    );

    frame.render_widget(
        Paragraph::new(label).style(bold_if(
            Style::default()
                .fg(theme.compact_mode_text_color(mode))
                .bg(mode_background),
            theme.status_bold,
        )),
        layout.mode,
    );

    if layout.separator.width > 0 {
        frame.render_widget(
            Paragraph::new(glyph).style(background.fg(mode_background)),
            layout.separator,
        );
    }

    if layout.information.width > 0 {
        let information = information(state);
        let information = match state.status_line_options.mode_position {
            ModePosition::Left => format!(" {information}"),
            ModePosition::Right => information,
        };
        frame.render_widget(
            Paragraph::new(information).style(background.fg(theme.inactive_title)),
            layout.information,
        );
    }
}

fn plain_layout(area: Rect, mode_width: u16, position: ModePosition) -> StatusLayout {
    match position {
        ModePosition::Left => {
            let remaining = area.width.saturating_sub(mode_width);
            let separator_width = 3u16.min(remaining);

            StatusLayout {
                mode: Rect::new(area.x, area.y, mode_width, 1),
                separator: Rect::new(area.x + mode_width, area.y, separator_width, 1),
                information: Rect::new(
                    area.x + mode_width + separator_width,
                    area.y,
                    remaining.saturating_sub(separator_width),
                    1,
                ),
            }
        }
        ModePosition::Right => {
            let mode_x = area.right().saturating_sub(mode_width);
            let information_width = area.width.saturating_sub(mode_width);
            let separator_width = 3u16.min(information_width);
            let separator_x = mode_x.saturating_sub(separator_width);

            StatusLayout {
                mode: Rect::new(mode_x, area.y, mode_width, 1),
                separator: Rect::new(separator_x, area.y, separator_width, 1),
                information: Rect::new(area.x, area.y, separator_x.saturating_sub(area.x), 1),
            }
        }
    }
}

fn compact_layout(
    area: Rect,
    label_width: u16,
    glyph: &str,
    position: ModePosition,
) -> StatusLayout {
    let label_width = label_width.min(area.width);
    match position {
        ModePosition::Left => {
            let cursor = area.x.saturating_add(label_width);
            let separator_width = Line::from(glyph)
                .width()
                .min(usize::from(area.right().saturating_sub(cursor)))
                as u16;
            let information_x = cursor.saturating_add(separator_width);

            StatusLayout {
                mode: Rect::new(area.x, area.y, label_width, 1),
                separator: Rect::new(cursor, area.y, separator_width, 1),
                information: Rect::new(
                    information_x,
                    area.y,
                    area.right().saturating_sub(information_x),
                    1,
                ),
            }
        }
        ModePosition::Right => {
            let label_x = area.right().saturating_sub(label_width);
            let separator_width = Line::from(glyph)
                .width()
                .min(usize::from(label_x.saturating_sub(area.x)))
                as u16;
            let separator_x = label_x.saturating_sub(separator_width);

            StatusLayout {
                mode: Rect::new(label_x, area.y, label_width, 1),
                separator: Rect::new(separator_x, area.y, separator_width, 1),
                information: Rect::new(area.x, area.y, separator_x.saturating_sub(area.x), 1),
            }
        }
    }
}

fn information(state: &AppState) -> String {
    format!(
        "{} {} Hidden: {}",
        state.protocol_label,
        if state.workspace.icon_mode == crate::icons::IconMode::None {
            "|"
        } else {
            "·"
        },
        if state.workspace.show_hidden {
            "on"
        } else {
            "off"
        }
    )
}

fn bold_if(style: Style, enabled: bool) -> Style {
    if enabled {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::terminal::TerminalSize;
    use crate::workspace::{PanelId, WorkspaceOptions};

    use super::*;

    fn state() -> AppState {
        AppState::new(
            PathBuf::from("."),
            "Kitty".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            WorkspaceOptions::default(),
        )
    }

    #[test]
    fn hidden_indicator_is_rendered_in_every_footer_style() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut state = state();
        let theme = Theme::emergency_monochrome();
        for style in [UiStyle::Framed, UiStyle::Borderless, UiStyle::Compact] {
            state.status_line_options.style = style;
            for position in [ModePosition::Left, ModePosition::Right] {
                state.status_line_options.mode_position = position;
                for show in [false, true] {
                    state.workspace.show_hidden = show;
                    let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
                    terminal
                        .draw(|frame| draw(frame, frame.area(), &state, &theme))
                        .unwrap();
                    let text: String = terminal
                        .backend()
                        .buffer()
                        .content()
                        .iter()
                        .map(|cell| cell.symbol())
                        .collect();
                    assert!(text.contains(if show { "Hidden: on" } else { "Hidden: off" }));
                }
            }
        }
    }

    #[test]
    fn application_and_finder_states_map_to_public_modes() {
        let mut state = state();
        assert_eq!(current_mode(&state), StatusMode::Normal);
        state.mode = AppMode::Command;
        assert_eq!(current_mode(&state), StatusMode::Command);
        state.mode = AppMode::Insert;
        assert_eq!(current_mode(&state), StatusMode::Insert);
        state.mode = AppMode::Search;
        assert_eq!(current_mode(&state), StatusMode::Search);
        state.mode = AppMode::Visual;
        assert_eq!(current_mode(&state), StatusMode::Visual);
        state.mode = AppMode::Confirm;
        assert_eq!(current_mode(&state), StatusMode::Normal);
        state.dialog_options.display = DialogDisplay::CommandLine;
        assert_eq!(current_mode(&state), StatusMode::Command);
        state.finder.open(PanelId::Left, PathBuf::from("."));
        assert_eq!(current_mode(&state), StatusMode::Insert);
        state.finder.mode = FinderMode::Searching;
        assert_eq!(current_mode(&state), StatusMode::Search);
    }
}
