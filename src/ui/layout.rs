use ratatui::layout::{Constraint, Layout, Margin, Rect};

use crate::status_line::UiStyle;
use crate::workspace::{PanelId, PanelStyle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewLayout {
    pub panel: Rect,
    pub inner: Rect,
    pub media: Rect,
    pub separator: Rect,
    pub metadata: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLayout {
    pub left_panel: Rect,
    pub right_panel: Rect,
    pub left_inner: Rect,
    pub right_inner: Rect,
    pub panel_gap: Rect,
    pub preview: Option<PreviewLayout>,
    pub status_line: Rect,
    pub command_line: Rect,
    pub bottom_spacer: Rect,
    pub bottom_frame: Rect,
    pub bottom_separator: Rect,
}

struct FooterLayout {
    content: Rect,
    status_line: Rect,
    command_line: Rect,
    bottom_spacer: Rect,
    bottom_frame: Rect,
    bottom_separator: Rect,
}

impl AppLayout {
    pub fn panel(self, panel: PanelId) -> Rect {
        match panel {
            PanelId::Left => self.left_panel,
            PanelId::Right => self.right_panel,
        }
    }

    pub fn inner(self, panel: PanelId) -> Rect {
        match panel {
            PanelId::Left => self.left_inner,
            PanelId::Right => self.right_inner,
        }
    }
}

pub fn resolve_layout(area: Rect, preview_panel: Option<PanelId>) -> AppLayout {
    resolve_layout_with_style(
        area,
        preview_panel,
        crate::preview::PreviewOptions::default().media_percent,
        UiStyle::Framed,
    )
}

pub fn resolve_layout_with_preview(
    area: Rect,
    preview_panel: Option<PanelId>,
    media_percent: u16,
) -> AppLayout {
    resolve_layout_with_style(area, preview_panel, media_percent, UiStyle::Framed)
}

pub fn resolve_layout_with_style(
    area: Rect,
    preview_panel: Option<PanelId>,
    media_percent: u16,
    style: UiStyle,
) -> AppLayout {
    resolve_layout_with_styles(
        area,
        preview_panel,
        media_percent,
        style,
        PanelStyle::Framed,
    )
}

pub fn resolve_layout_with_styles(
    area: Rect,
    preview_panel: Option<PanelId>,
    media_percent: u16,
    style: UiStyle,
    panel_style: PanelStyle,
) -> AppLayout {
    let footer = footer_layout(area, style);
    let (left_panel, panel_gap, right_panel) = panel_layout(footer.content, panel_style);
    let left_inner = panel_inner(left_panel, panel_style);
    let right_inner = panel_inner(right_panel, panel_style);
    let preview = preview_panel.map(|panel_id| {
        let panel = match panel_id {
            PanelId::Left => left_panel,
            PanelId::Right => right_panel,
        };
        preview_layout(panel, media_percent, panel_style)
    });

    AppLayout {
        left_panel,
        right_panel,
        left_inner,
        right_inner,
        panel_gap,
        preview,
        status_line: footer.status_line,
        command_line: footer.command_line,
        bottom_spacer: footer.bottom_spacer,
        bottom_frame: footer.bottom_frame,
        bottom_separator: footer.bottom_separator,
    }
}

pub(super) fn preview_layout(
    panel: Rect,
    media_percent: u16,
    panel_style: PanelStyle,
) -> PreviewLayout {
    let inner = panel_inner(panel, panel_style);

    let separator_height = u16::from(inner.height > 0);
    let content_height = inner.height.saturating_sub(separator_height);
    let media_height = if content_height == 0 {
        0
    } else {
        ((u32::from(content_height) * u32::from(media_percent.clamp(20, 80)) / 100) as u16)
            .clamp(1, content_height)
    };

    let media = Rect::new(inner.x, inner.y, inner.width, media_height);
    let separator = Rect::new(
        inner.x,
        inner.y.saturating_add(media_height),
        inner.width,
        separator_height,
    );
    let metadata_height = content_height.saturating_sub(media_height);
    let metadata = Rect::new(
        inner.x,
        separator.y.saturating_add(separator_height),
        inner.width,
        metadata_height,
    );

    PreviewLayout {
        panel,
        inner,
        media,
        separator,
        metadata,
    }
}

fn footer_layout(area: Rect, style: UiStyle) -> FooterLayout {
    match style {
        UiStyle::Framed => {
            let [content, bottom_frame] =
                Layout::vertical([Constraint::Min(1), Constraint::Length(5)]).areas(area);
            let inner = bottom_frame.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            let [status_row, bottom_separator, command_row] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(inner);
            let content_margin = Margin {
                horizontal: 1,
                vertical: 0,
            };

            FooterLayout {
                content,
                status_line: status_row.inner(content_margin),
                command_line: command_row.inner(content_margin),
                bottom_spacer: Rect::new(area.x, area.bottom(), area.width, 0),
                bottom_frame,
                bottom_separator,
            }
        }
        UiStyle::Borderless => {
            let [content, status_line, command_line] = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

            FooterLayout {
                content,
                status_line,
                command_line,
                bottom_spacer: Rect::new(area.x, area.bottom(), area.width, 0),
                bottom_frame: Rect::new(area.x, area.bottom(), area.width, 0),
                bottom_separator: Rect::new(area.x, area.bottom(), area.width, 0),
            }
        }
        UiStyle::Compact => {
            let [content, bottom_separator, status_line, command_line] = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

            FooterLayout {
                content,
                status_line,
                command_line,
                bottom_spacer: Rect::new(area.x, area.bottom(), area.width, 0),
                bottom_frame: Rect::new(area.x, area.bottom(), area.width, 0),
                bottom_separator,
            }
        }
    }
}

fn panel_layout(content: Rect, panel_style: PanelStyle) -> (Rect, Rect, Rect) {
    match panel_style {
        PanelStyle::Framed => {
            let [left_panel, right_panel] =
                Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .areas(content);

            (
                left_panel,
                Rect::new(left_panel.right(), content.y, 0, content.height),
                right_panel,
            )
        }
        PanelStyle::Borderless | PanelStyle::Compact => {
            let [left_panel, panel_gap, right_panel] = Layout::horizontal([
                Constraint::Ratio(1, 2),
                Constraint::Length(1),
                Constraint::Ratio(1, 2),
            ])
            .areas(content);

            (left_panel, panel_gap, right_panel)
        }
    }
}

fn panel_inner(panel: Rect, panel_style: PanelStyle) -> Rect {
    match panel_style {
        PanelStyle::Framed => panel.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }),
        PanelStyle::Borderless | PanelStyle::Compact => Rect::new(
            panel.x,
            panel.y.saturating_add(u16::from(panel.height > 0)),
            panel.width,
            panel.height.saturating_sub(1),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_split_even_and_odd_widths_without_content_influence() {
        let even = resolve_layout(Rect::new(0, 0, 100, 30), None);
        assert_eq!(even.left_panel.width, 50);
        assert_eq!(even.right_panel.width, 50);

        let odd = resolve_layout(Rect::new(0, 0, 101, 30), None);
        assert!(odd.left_panel.width.abs_diff(odd.right_panel.width) <= 1);
        assert_eq!(odd.left_panel.width + odd.right_panel.width, 101);
    }

    #[test]
    fn preview_has_independent_media_separator_and_metadata_regions() {
        let layout =
            resolve_layout_with_preview(Rect::new(0, 0, 100, 30), Some(PanelId::Right), 50);
        let preview = layout.preview.unwrap();
        assert_eq!(preview.panel, layout.right_panel);
        assert_eq!(preview.separator.height, 1);
        assert_eq!(
            preview.media.height + preview.separator.height + preview.metadata.height,
            preview.inner.height
        );
        assert!(preview.media.height.abs_diff(preview.metadata.height) <= 1);
    }

    #[test]
    fn preview_layout_is_safe_from_zero_through_five_rows() {
        for height in 0..=5 {
            let layout =
                resolve_layout_with_preview(Rect::new(0, 0, 10, height), Some(PanelId::Left), 80);
            let preview = layout.preview.unwrap();
            assert!(preview.media.bottom() <= preview.inner.bottom());
            assert!(preview.metadata.bottom() <= preview.inner.bottom());
        }
    }

    #[test]
    fn framed_footer_wraps_plain_rows_and_a_separator_in_one_frame() {
        let layout = resolve_layout(Rect::new(0, 0, 80, 24), None);
        assert_eq!(layout.bottom_frame, Rect::new(0, 19, 80, 5));
        assert_eq!(layout.status_line, Rect::new(2, 20, 76, 1));
        assert_eq!(layout.bottom_separator, Rect::new(1, 21, 78, 1));
        assert_eq!(layout.command_line, Rect::new(2, 22, 76, 1));
        assert_eq!(layout.left_panel.bottom(), layout.bottom_frame.top());
        assert_eq!(layout.bottom_frame.bottom(), 24);
    }

    #[test]
    fn borderless_footer_reserves_only_status_and_command_rows() {
        let layout =
            resolve_layout_with_style(Rect::new(0, 0, 80, 24), None, 50, UiStyle::Borderless);
        assert_eq!(layout.status_line.height, 1);
        assert_eq!(layout.command_line.height, 1);
        assert_eq!(layout.bottom_spacer.height, 0);
        assert_eq!(layout.left_panel.bottom(), layout.status_line.top());
        assert_eq!(layout.status_line.bottom(), layout.command_line.top());
        assert_eq!(layout.command_line.bottom(), 24);
        assert_eq!(layout.bottom_frame.height, 0);
        assert_eq!(layout.bottom_separator.height, 0);
    }

    #[test]
    fn compact_footer_adds_only_a_top_rule() {
        let layout = resolve_layout_with_style(Rect::new(0, 0, 80, 24), None, 50, UiStyle::Compact);
        assert_eq!(layout.bottom_separator, Rect::new(0, 21, 80, 1));
        assert_eq!(layout.status_line, Rect::new(0, 22, 80, 1));
        assert_eq!(layout.command_line, Rect::new(0, 23, 80, 1));
        assert!(layout.bottom_frame.is_empty());
        assert!(layout.bottom_spacer.is_empty());
        assert_eq!(layout.command_line.bottom(), 24);
    }

    #[test]
    fn borderless_panels_reserve_only_the_title_row() {
        let layout = resolve_layout_with_styles(
            Rect::new(0, 0, 80, 24),
            Some(PanelId::Right),
            50,
            UiStyle::Borderless,
            PanelStyle::Borderless,
        );
        assert_eq!(layout.left_panel.width + layout.right_panel.width, 79);
        assert_eq!(layout.panel_gap.width, 1);
        assert_eq!(layout.panel_gap.x, layout.left_panel.right());
        assert_eq!(layout.right_panel.x, layout.panel_gap.right());
        assert_eq!(layout.left_panel.height, 22);
        assert_eq!(layout.left_inner.height, 21);
        let preview = layout.preview.unwrap();
        assert_eq!(preview.panel, layout.right_panel);
        assert_eq!(preview.inner.y, 1);
        assert_eq!(preview.inner.height, 21);
    }

    #[test]
    fn compact_panels_keep_a_single_separator_column() {
        let layout = resolve_layout_with_styles(
            Rect::new(0, 0, 80, 24),
            None,
            50,
            UiStyle::Borderless,
            PanelStyle::Compact,
        );
        assert_eq!(layout.panel_gap.width, 1);
        assert_eq!(layout.panel_gap.height, 22);
        assert_eq!(layout.left_panel.width + layout.right_panel.width, 79);
    }
}
