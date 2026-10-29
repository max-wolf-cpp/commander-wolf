use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::preview::ViewerStatus;
use crate::theme::Theme;

pub fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    if !state.viewer.is_open() {
        return;
    }

    let Some(layout) = state.viewer.layout else {
        draw_probe_status(frame, state, theme);
        return;
    };

    super::floating::clear(frame, layout.modal, theme.float_background);
    let background = Style::default().bg(theme.float_background.into());
    super::chrome::draw_titled_block(
        frame,
        layout.modal,
        Block::default()
            .style(background)
            .borders(Borders::ALL)
            .border_set(theme.border_set())
            .border_style(Style::default().fg(theme.active_border.into())),
        &state.viewer.title,
        Style::default()
            .fg(theme.active_title.into())
            .add_modifier(if theme.title_bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        background,
    );

    if state.viewer.status == ViewerStatus::Ready {
        super::mark_media_viewport_skipped(frame.buffer_mut(), layout.media);
    } else if let ViewerStatus::Unavailable(message) = &state.viewer.status {
        frame.render_widget(
            Paragraph::new(message.as_str())
                .centered()
                .style(background.fg(theme.preview_unavailable.into())),
            layout.media,
        );
    }
}

fn draw_probe_status(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let area = frame.area();
    let width = area.width.min(50);
    let height = area.height.min(5);
    if width == 0 || height == 0 {
        return;
    }

    let modal = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    super::floating::clear(frame, modal, theme.float_background);
    frame.render_widget(
        Paragraph::new("Preparing image…")
            .centered()
            .style(
                Style::default()
                    .bg(theme.float_background.into())
                    .fg(theme.text.into()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(theme.border_set())
                    .title(state.viewer.title.as_str())
                    .border_style(Style::default().fg(theme.active_border.into())),
            ),
        modal,
    );
}
