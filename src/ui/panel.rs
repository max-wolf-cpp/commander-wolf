use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::directory_edit::DirectoryEditState;
use crate::icons::IconMode;
use crate::search::SearchState;
use crate::theme::Theme;
use crate::workspace::{LineNumberMode, PanelId, PanelState, PanelStyle};

use super::chrome::{draw_borderless_title, draw_titled_block};

mod directory_edit;
mod rows;

pub struct PanelOverlay<'a> {
    pub search: (PanelId, &'a SearchState),
    pub edit: Option<&'a DirectoryEditState>,
    pub visual_anchor: Option<usize>,
    pub icon_mode: IconMode,
    pub color_file_names: bool,
    pub panel_style: PanelStyle,
}

pub fn draw_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: &PanelState,
    line_numbers: LineNumberMode,
    active: bool,
    overlay: PanelOverlay<'_>,
    theme: &Theme,
) {
    let inner = draw_chrome(frame, area, panel, active, &overlay, theme);

    if let Some(edit) = overlay.edit {
        directory_edit::draw(
            frame,
            inner,
            edit,
            line_numbers,
            overlay.icon_mode,
            theme,
            overlay.color_file_names,
        );
        return;
    }

    if panel.entries.is_empty() {
        frame.render_widget(
            List::new([ListItem::new("(empty)")]).style(
                Style::default()
                    .fg(theme.inactive_title.into())
                    .bg(theme.base.into()),
            ),
            inner,
        );
        return;
    }

    let first = panel.viewport.scroll.min(panel.entries.len());
    let last = first
        .saturating_add(usize::from(inner.height))
        .min(panel.entries.len());
    let items = panel.entries[first..last]
        .iter()
        .enumerate()
        .map(|(visible_index, entry)| {
            rows::entry_item(
                entry,
                first + visible_index,
                panel,
                line_numbers,
                active,
                &overlay,
                theme,
            )
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(items).style(Style::default().bg(theme.base.into())),
        inner,
    );
}

fn draw_chrome(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: &PanelState,
    active: bool,
    overlay: &PanelOverlay<'_>,
    theme: &Theme,
) -> Rect {
    let background = Style::default().bg(theme.base.into());
    let title = panel.directory.display().to_string();

    match overlay.panel_style {
        PanelStyle::Framed => {
            let border_color = if active {
                theme.active_border
            } else {
                theme.inactive_border
            };
            let border_style = Style::default().fg(border_color.into());
            let border_style = if !active && theme.dim_secondary {
                border_style.add_modifier(ratatui::style::Modifier::DIM)
            } else {
                border_style
            };
            let block = Block::default()
                .style(background)
                .borders(Borders::ALL)
                .border_set(theme.panel_border_set())
                .border_style(border_style);

            let title_color = if active {
                theme.framed_active_title_text.unwrap_or(theme.active_title)
            } else {
                theme.inactive_title
            };
            let title_background = if active {
                theme.framed_active_title_background.unwrap_or(theme.base)
            } else {
                theme.base
            };
            draw_titled_block(
                frame,
                area,
                block,
                &title,
                super::chrome::bold_if(
                    Style::default()
                        .fg(title_color.into())
                        .bg(title_background.into()),
                    theme.title_bold,
                ),
                background,
            )
        }
        PanelStyle::Borderless | PanelStyle::Compact => {
            frame.render_widget(Block::default().style(background), area);
            draw_borderless_title(
                frame,
                Rect::new(area.x, area.y, area.width, u16::from(area.height > 0)),
                &title,
                if active {
                    theme.borderless_active_title_background
                } else {
                    theme.borderless_inactive_title_background
                },
                background,
                theme,
                overlay.icon_mode,
                if active {
                    theme.borderless_title_text
                } else {
                    theme
                        .borderless_inactive_title_text
                        .unwrap_or(theme.borderless_title_text)
                },
            );

            Rect::new(
                area.x,
                area.y.saturating_add(u16::from(area.height > 0)),
                area.width,
                area.height.saturating_sub(1),
            )
        }
    }
}

#[cfg(test)]
mod tests;
