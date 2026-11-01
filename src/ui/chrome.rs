use std::borrow::Cow;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

pub(super) fn draw_titled_block(
    frame: &mut Frame<'_>,
    area: Rect,
    block: Block<'_>,
    title: &str,
    title_style: Style,
    background: Style,
) -> Rect {
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Keep three border cells and a corner on each side of the padded title.
    if area.width >= 10 && area.height > 0 {
        let title = fit_title(title, usize::from(area.width - 10));
        let title = Line::styled(format!(" {title} "), title_style);
        let title_width = title.width() as u16;
        let title_area = Rect::new(area.x.saturating_add(4), area.y, title_width, 1);
        frame.render_widget(Paragraph::new(title).style(background), title_area);
    }

    inner
}

fn fit_title(title: &str, width: usize) -> Cow<'_, str> {
    if title.width() <= width {
        return Cow::Borrowed(title);
    }

    let dots = width.min(3);
    let prefix_width = width - dots;
    let mut shortened = String::new();
    let mut used = 0;
    for grapheme in title.graphemes(true) {
        let grapheme_width = grapheme.width();
        if used + grapheme_width > prefix_width {
            break;
        }

        shortened.push_str(grapheme);
        used += grapheme_width;
    }
    // A wide grapheme may leave one cell unused; keep the dots aligned at the right edge.
    shortened.push_str(&" ".repeat(prefix_width - used));
    shortened.push_str(&"..."[..dots]);

    Cow::Owned(shortened)
}

pub(super) fn draw_borderless_title(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    title_background: Color,
    panel_background: Style,
    theme: &Theme,
    title_text_color: Color,
) {
    if area.is_empty() {
        return;
    }

    frame.render_widget(Block::default().style(panel_background), area);

    let left_glyph = if theme.decorations {
        theme.borderless_title_left.as_str()
    } else {
        ""
    };
    let right_glyph = if theme.decorations {
        theme.borderless_title_right.as_str()
    } else {
        ""
    };
    let left_width = Line::from(left_glyph).width().min(usize::from(area.width)) as u16;
    let right_width = Line::from(right_glyph)
        .width()
        .min(usize::from(area.width.saturating_sub(left_width))) as u16;

    if left_width > 0 {
        frame.render_widget(
            Paragraph::new(left_glyph).style(panel_background.fg(title_background)),
            Rect::new(area.x, area.y, left_width, 1),
        );
    }

    let center_x = area.x.saturating_add(left_width);
    let center_width = area
        .width
        .saturating_sub(left_width)
        .saturating_sub(right_width);
    if center_width > 0 {
        frame.render_widget(
            Paragraph::new(format!("  {title}")).style(bold_if(
                Style::default().fg(title_text_color).bg(title_background),
                theme.title_bold,
            )),
            Rect::new(center_x, area.y, center_width, 1),
        );
    }

    if right_width > 0 {
        frame.render_widget(
            Paragraph::new(right_glyph).style(panel_background.fg(title_background)),
            Rect::new(
                area.right().saturating_sub(right_width),
                area.y,
                right_width,
                1,
            ),
        );
    }
}

pub(super) fn bold_if(style: Style, enabled: bool) -> Style {
    if enabled {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}
