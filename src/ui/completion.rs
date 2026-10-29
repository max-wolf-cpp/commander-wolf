use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{AppMode, AppState};
use crate::commands::completion::CommandCompletion;
use crate::theme::Theme;

pub(super) fn draw(frame: &mut Frame<'_>, command_line: Rect, state: &AppState, theme: &Theme) {
    let Some(completion) = state
        .command_completion
        .as_ref()
        .filter(|_| state.mode == AppMode::Command)
    else {
        return;
    };

    let area = popup_area(frame.area(), command_line, completion);
    if area.width < 5 || area.height < 3 {
        return;
    }

    let inner = draw_chrome(frame, area, completion, theme);
    draw_items(frame, inner, completion, theme);
    draw_scrollbar(
        frame,
        area,
        inner,
        completion,
        state.workspace.icon_mode,
        theme,
    );
}

fn popup_area(screen: Rect, command_line: Rect, completion: &CommandCompletion) -> Rect {
    let arguments = completion.is_argument();
    let name_width = completion
        .items
        .iter()
        .map(|item| Line::raw(&item.name).width())
        .max()
        .unwrap_or(0);
    let description_width = if arguments {
        0
    } else {
        completion
            .items
            .iter()
            .map(|item| Line::raw(&item.description).width())
            .max()
            .unwrap_or(0)
    };
    let width = name_width
        .saturating_add(description_width)
        .saturating_add(6)
        .max(if arguments { 10 } else { 20 })
        .min(usize::from(screen.right().saturating_sub(command_line.x))) as u16;
    let height = completion
        .items
        .len()
        .saturating_add(2)
        .min(usize::from(screen.height / 2))
        .min(usize::from(command_line.y.saturating_sub(screen.y))) as u16;
    Rect::new(
        command_line.x,
        command_line.y.saturating_sub(height),
        width,
        height,
    )
}

fn draw_chrome(
    frame: &mut Frame<'_>,
    area: Rect,
    completion: &CommandCompletion,
    theme: &Theme,
) -> Rect {
    let background = Style::default()
        .bg(theme.float_background.into())
        .fg(theme.text.into());
    super::floating::clear(frame, area, theme.float_background);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme.border_set())
        .border_style(Style::default().fg(theme.active_border.into()))
        .style(background);
    if completion.is_argument() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        inner
    } else {
        super::chrome::draw_titled_block(
            frame,
            area,
            block,
            "Commands",
            Style::default()
                .fg(theme.active_title.into())
                .add_modifier(if theme.title_bold {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            background,
        )
    }
}

fn draw_items(frame: &mut Frame<'_>, inner: Rect, completion: &CommandCompletion, theme: &Theme) {
    let background = Style::default()
        .bg(theme.float_background.into())
        .fg(theme.text.into());
    let content = inner.inner(Margin::new(1, 0));
    let rows = usize::from(content.height);
    let name_width = name_column_width(content.width, completion);
    let first = completion.selected.saturating_sub(rows - 1);

    for (row, item) in completion.items.iter().skip(first).take(rows).enumerate() {
        let y = content.y + row as u16;
        let selected = first + row == completion.selected;
        let style = if selected {
            background
                .fg(theme.selected_text.into())
                .bg(theme.selected.into())
        } else {
            background
        };
        frame.render_widget(
            Block::default().style(style),
            Rect::new(inner.x, y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(item.name.as_str()).style(if selected {
                style
                    .fg(if theme.selected_text != theme.text {
                        theme.selected_text.into()
                    } else {
                        theme.active_title.into()
                    })
                    .add_modifier(Modifier::BOLD)
            } else {
                style
            }),
            Rect::new(content.x, y, name_width, 1),
        );

        if !completion.is_argument() {
            let offset = (name_width + 2).min(content.width);
            frame.render_widget(
                Paragraph::new(item.description.as_str()).style(style.fg(
                    if selected && theme.selected_text != theme.text {
                        theme.selected_text.into()
                    } else {
                        theme.inactive_title.into()
                    },
                )),
                Rect::new(content.x + offset, y, content.width - offset, 1),
            );
        }
    }
}

fn name_column_width(width: u16, completion: &CommandCompletion) -> u16 {
    if completion.is_argument() {
        return width;
    }

    let description_width = completion
        .items
        .iter()
        .map(|item| Line::raw(&item.description).width())
        .max()
        .unwrap_or(0);
    let name_limit = usize::from(width)
        .saturating_sub(description_width.saturating_add(2))
        .max(usize::from(width / 2).max(1));
    completion
        .items
        .iter()
        .map(|item| Line::raw(&item.name).width())
        .max()
        .unwrap_or(0)
        .min(name_limit) as u16
}

fn draw_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    inner: Rect,
    completion: &CommandCompletion,
    icon_mode: crate::icons::IconMode,
    theme: &Theme,
) {
    let rows = usize::from(inner.height);
    if completion.items.len() <= rows {
        return;
    }

    let first = completion.selected.saturating_sub(rows - 1);
    let background = Style::default()
        .bg(theme.float_background.into())
        .fg(theme.text.into());
    let thumb_height = (rows * rows / completion.items.len()).max(1);
    let thumb_start = first * (rows - thumb_height) / (completion.items.len() - rows);
    let ascii = icon_mode == crate::icons::IconMode::None;
    for row in 0..rows {
        let thumb = (thumb_start..thumb_start + thumb_height).contains(&row);
        let symbol = match (ascii, thumb) {
            (true, true) => "#",
            (true, false) => "|",
            (false, true) => "┃",
            (false, false) => "│",
        };
        frame.render_widget(
            Paragraph::new(symbol).style(
                background.fg(if thumb {
                    theme.active_border
                } else {
                    theme.line_number
                }
                .into()),
            ),
            Rect::new(area.right() - 1, inner.y + row as u16, 1, 1),
        );
    }
}

#[cfg(test)]
mod tests;
