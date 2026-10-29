use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{AppMode, AppState};
use crate::dialogs::DialogDisplay;
use crate::theme::Theme;

mod content;

pub fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    if state.mode != AppMode::Confirm || state.dialog_options.display != DialogDisplay::Float {
        return;
    }

    let content = content::build(state, theme, false);
    let preferred_width = content
        .body
        .iter()
        .chain(&content.actions)
        .map(ratatui::text::Line::width)
        .max()
        .unwrap_or_default()
        .saturating_add(2)
        .max(76);
    let colors = theme.dialog_colors(content.kind);
    let background = Style::default().bg(colors.background);
    let body = Paragraph::new(content.body)
        .style(background.fg(colors.text))
        .wrap(Wrap { trim: false });
    let actions = Paragraph::new(content.actions)
        .style(background.fg(colors.text))
        .wrap(Wrap { trim: false });
    let area = centered_dialog(frame.area(), preferred_width, &body, &actions);
    if area.is_empty() {
        return;
    }

    super::floating::clear(frame, area, colors.background);
    let accent = colors.accent;
    let block = Block::default()
        .style(background)
        .borders(Borders::ALL)
        .border_set(theme.border_set())
        .border_style(Style::default().fg(accent.into()));
    let inner = block.inner(area);
    super::chrome::draw_titled_block(
        frame,
        area,
        block,
        content.title,
        Style::default()
            .fg(accent.into())
            .add_modifier(Modifier::BOLD),
        background,
    );

    let action_rows = actions.line_count(inner.width);
    let action_height = action_rows.min(usize::from(inner.height)) as u16;
    let body_area = Rect::new(inner.x, inner.y, inner.width, inner.height - action_height);
    let actions_area = Rect::new(inner.x, body_area.bottom(), inner.width, action_height);

    // Keep the final choices visible even when the terminal cannot fit all details.
    let action_scroll =
        u16::try_from(action_rows.saturating_sub(usize::from(action_height))).unwrap_or(u16::MAX);
    frame.render_widget(body, body_area);
    frame.render_widget(actions.scroll((action_scroll, 0)), actions_area);
}

pub(super) fn command_line_text(state: &AppState) -> String {
    let content = content::build(state, &Theme::emergency_monochrome(), true);
    let lines = content
        .body
        .into_iter()
        .chain(content.actions)
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| span.content)
                .collect::<String>()
        })
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_owned())
        .collect::<Vec<_>>();

    format!("{}: {}", content.title, lines.join(" | "))
}

fn centered_dialog(
    terminal: Rect,
    preferred_width: usize,
    body: &Paragraph<'_>,
    actions: &Paragraph<'_>,
) -> Rect {
    if terminal.is_empty() {
        return terminal;
    }

    let width = terminal
        .width
        .saturating_sub(2)
        .max(1)
        .min(u16::try_from(preferred_width).unwrap_or(u16::MAX));
    let inner_width = width.saturating_sub(2);
    let content_height = body
        .line_count(inner_width)
        .saturating_add(actions.line_count(inner_width))
        .saturating_add(2);
    let height = terminal
        .height
        .saturating_sub(2)
        .max(1)
        .min(u16::try_from(content_height).unwrap_or(u16::MAX));

    Rect::new(
        terminal.x + terminal.width.saturating_sub(width) / 2,
        terminal.y + terminal.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests;
