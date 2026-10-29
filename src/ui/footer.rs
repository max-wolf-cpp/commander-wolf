use ratatui::Frame;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

use crate::app::AppState;
use crate::theme::Theme;

use super::{AppLayout, command_line, completion, separator_horizontal, status_line};

pub(super) fn draw(frame: &mut Frame<'_>, layout: AppLayout, state: &AppState, theme: &Theme) {
    if !layout.bottom_frame.is_empty() {
        let mode = status_line::current_mode(state);
        frame.render_widget(
            Block::default()
                .style(Style::default().bg(theme.status_background.into()))
                .borders(Borders::ALL)
                .border_set(theme.footer_border_set())
                .border_style(Style::default().fg(theme.framed_footer_border_color(mode).into())),
            layout.bottom_frame,
        );
    }

    let compact = state.status_line_options.style == crate::status_line::UiStyle::Compact;
    if !layout.bottom_separator.is_empty()
        && (compact || theme.separator_style != crate::theme::SeparatorStyle::Off)
    {
        let separator = if compact {
            let horizontal = theme.footer_border_set().horizontal_top;
            horizontal.repeat(usize::from(layout.bottom_separator.width))
        } else {
            let horizontal = separator_horizontal(theme);
            let horizontal_width = layout.bottom_separator.width.saturating_sub(2);
            format!(" {} ", horizontal.repeat(usize::from(horizontal_width)))
        };
        let separator_color =
            if state.status_line_options.style == crate::status_line::UiStyle::Framed {
                theme.line_number
            } else {
                theme.status_border
            };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(separator).style(
                Style::default()
                    .fg(separator_color.into())
                    .bg(theme.status_background.into()),
            ),
            layout.bottom_separator,
        );
    }

    status_line::draw(frame, layout.status_line, state, theme);
    command_line::draw(frame, layout.command_line, state, theme);

    if !layout.bottom_spacer.is_empty() {
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.status_background.into())),
            layout.bottom_spacer,
        );
    }

    completion::draw(frame, layout.command_line, state, theme);
}
