use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::preview::MetadataRow;
use crate::theme::Theme;
use crate::workspace::PanelStyle;

use super::layout::PreviewLayout;

mod audio;
mod content;
mod listing;
mod text;

pub(super) struct PreviewChrome<'a> {
    pub title: &'a str,
    pub panel_style: PanelStyle,
    pub background_color: Color,
    pub border_color: Color,
    pub title_color: Color,
}

pub fn draw(frame: &mut Frame<'_>, layout: PreviewLayout, state: &AppState, theme: &Theme) {
    let title = if state.preview.title.is_empty() {
        "Preview"
    } else {
        state.preview.title.as_str()
    };
    draw_with_chrome(
        frame,
        layout,
        state,
        theme,
        PreviewChrome {
            title,
            panel_style: state.workspace.panel_style,
            background_color: theme.preview_background,
            border_color: theme.active_border,
            title_color: theme
                .preview_title_text
                .unwrap_or(theme.borderless_preview_title_background),
        },
    );
}

pub(super) fn draw_with_chrome(
    frame: &mut Frame<'_>,
    layout: PreviewLayout,
    state: &AppState,
    theme: &Theme,
    chrome: PreviewChrome<'_>,
) {
    let background = Style::default().bg(chrome.background_color.into());
    draw_chrome(frame, layout, state, theme, &chrome, background);
    content::draw(frame, layout.media, state, theme, background);

    let separator = layout.separator.inner(Margin::new(1, 0));
    if !separator.is_empty() && theme.separator_style != crate::theme::SeparatorStyle::Off {
        frame.render_widget(
            Paragraph::new(super::separator_horizontal(theme).repeat(usize::from(separator.width)))
                .style(background.fg(theme.preview_separator.into())),
            separator,
        );
    }

    draw_metadata(
        frame,
        layout.metadata.inner(Margin::new(1, 0)),
        state,
        theme,
        background,
    );
}

fn draw_chrome(
    frame: &mut Frame<'_>,
    layout: PreviewLayout,
    state: &AppState,
    theme: &Theme,
    chrome: &PreviewChrome<'_>,
    background: Style,
) {
    let title_style =
        Style::default()
            .fg(chrome.title_color.into())
            .add_modifier(if theme.title_bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
    match chrome.panel_style {
        PanelStyle::Framed => {
            super::chrome::draw_titled_block(
                frame,
                layout.panel,
                Block::default()
                    .style(background)
                    .borders(Borders::ALL)
                    .border_set(theme.panel_border_set())
                    .border_style(Style::default().fg(chrome.border_color.into())),
                chrome.title,
                title_style,
                background,
            );
        }
        PanelStyle::Borderless | PanelStyle::Compact => {
            frame.render_widget(Block::default().style(background), layout.panel);
            super::chrome::draw_borderless_title(
                frame,
                Rect::new(
                    layout.panel.x,
                    layout.panel.y,
                    layout.panel.width,
                    u16::from(layout.panel.height > 0),
                ),
                chrome.title,
                theme.borderless_preview_title_background,
                background,
                theme,
                state.workspace.icon_mode,
                theme
                    .preview_title_text
                    .unwrap_or(theme.borderless_title_text),
            );
        }
    }
}

fn draw_metadata(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    background: Style,
) {
    let layout = state.preview.metadata_layout(area.width);
    for (offset, row) in layout
        .rows
        .into_iter()
        .skip(state.preview.metadata_viewport.offset)
        .take(usize::from(area.height))
        .enumerate()
    {
        let row_area = Rect::new(area.x, area.y + offset as u16, area.width, 1);
        match row {
            MetadataRow::Field { label, value } => {
                frame.render_widget(
                    Paragraph::new(label).style(background.fg(theme.preview_metadata_label)),
                    Rect::new(row_area.x, row_area.y, layout.label_width, 1),
                );
                frame.render_widget(
                    Paragraph::new(value).style(background.fg(theme.preview_metadata_value)),
                    Rect::new(
                        row_area.x + layout.label_width,
                        row_area.y,
                        row_area.width.saturating_sub(layout.label_width),
                        1,
                    ),
                );
            }
            MetadataRow::Separator => frame.render_widget(
                Paragraph::new(super::separator_horizontal(theme).repeat(usize::from(area.width)))
                    .style(background.fg(theme.preview_separator)),
                row_area,
            ),
        }
    }
}
