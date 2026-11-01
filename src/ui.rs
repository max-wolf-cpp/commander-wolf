use ratatui::Frame;
use ratatui::buffer::CellDiffOption;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};

use crate::app::AppState;
use crate::theme::Theme;
use crate::workspace::{PanelId, PanelStyle};

pub use directory_tree::visible_rows as tree_visible_rows;
pub use finder::{preview_layout as finder_preview_layout, visible_rows as finder_visible_rows};
pub use help::visible_rows as help_visible_rows;
pub use layout::{
    AppLayout, PreviewLayout, resolve_layout, resolve_layout_with_preview,
    resolve_layout_with_style, resolve_layout_with_styles,
};

mod chrome;
mod command_line;
mod completion;
mod dialog;
mod directory_tree;
mod finder;
mod floating;
mod footer;
mod help;
mod layout;
mod operation_progress;
mod panel;
mod preview;
mod rename;
mod shell;
mod status_line;
mod viewer;

pub fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let preview_panel = (!state.finder.is_open() && !state.directory_tree.open)
        .then(|| state.workspace.preview_panel())
        .flatten();
    let layout = resolve_layout_with_styles(
        frame.area(),
        preview_panel,
        state.preview_options.media_percent,
        state.status_line_options.style,
        state.workspace.panel_style,
    );

    draw_workspace(frame, layout, preview_panel, state, theme);
    footer::draw(frame, layout, state, theme);
    draw_overlays(frame, state, theme);
}

pub fn mark_media_viewport_skipped(
    buffer: &mut ratatui::buffer::Buffer,
    region: ratatui::layout::Rect,
) {
    for row in region.top()..region.bottom() {
        for column in region.left()..region.right() {
            buffer[(column, row)].set_diff_option(CellDiffOption::Skip);
        }
    }
}

fn draw_workspace(
    frame: &mut Frame<'_>,
    layout: AppLayout,
    preview_panel: Option<PanelId>,
    state: &AppState,
    theme: &Theme,
) {
    draw_panel_gap(frame, layout, state.workspace.panel_style, theme);

    for panel_id in [PanelId::Left, PanelId::Right] {
        if preview_panel == Some(panel_id) {
            if let Some(preview_layout) = layout.preview {
                preview::draw(frame, preview_layout, state, theme);
            }
        } else {
            panel::draw_panel(
                frame,
                layout.panel(panel_id),
                state.workspace.panel(panel_id),
                state.workspace.line_number_mode,
                state.workspace.active_panel == panel_id,
                panel::PanelOverlay {
                    search: (panel_id, &state.search),
                    edit: state
                        .directory_edit
                        .active
                        .then_some(&state.directory_edit)
                        .filter(|edit| edit.panel == panel_id),
                    visual_anchor: (state.visual_selection.panel == Some(panel_id))
                        .then_some(state.visual_selection.anchor),
                    icon_mode: state.workspace.icon_mode,
                    color_file_names: state.workspace.color_file_names,
                    panel_style: state.workspace.panel_style,
                },
                theme,
            );
        }
    }
}

fn draw_panel_gap(
    frame: &mut Frame<'_>,
    layout: AppLayout,
    panel_style: PanelStyle,
    theme: &Theme,
) {
    if layout.panel_gap.is_empty() {
        return;
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(theme.base)),
        layout.panel_gap,
    );

    if panel_style == PanelStyle::Compact && layout.panel_gap.height > 1 {
        frame.render_widget(
            Block::default()
                .style(Style::default().bg(theme.base))
                .borders(Borders::LEFT)
                .border_set(theme.panel_border_set())
                .border_style(Style::default().fg(theme.inactive_border)),
            ratatui::layout::Rect::new(
                layout.panel_gap.x,
                layout.panel_gap.y + 1,
                layout.panel_gap.width,
                layout.panel_gap.height - 1,
            ),
        );
    }
}

fn draw_overlays(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    if state.finder.is_open() {
        finder::draw(frame, state, theme);
    }

    if state.directory_tree.open {
        directory_tree::draw(frame, state, theme);
    }

    dialog::draw(frame, state, theme);
    rename::draw(frame, state, theme);
    viewer::draw(frame, state, theme);
    shell::draw(frame, state, theme);
    help::draw(frame, state, theme);
    operation_progress::draw(frame, state, theme);
}

pub(super) fn separator_horizontal(theme: &Theme) -> &'static str {
    theme.separator_style.horizontal()
}

#[cfg(test)]
mod tests;
