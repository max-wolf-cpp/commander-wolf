use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

use crate::directory_edit::{DirectoryEditState, EditLine};
use crate::icons::{EntryIconKind, IconMode, resolve_for_theme};
use crate::theme::Theme;
use crate::workspace::LineNumberMode;

use super::rows::{line_number, line_number_span};

pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    edit: &DirectoryEditState,
    line_numbers: LineNumberMode,
    icon_mode: IconMode,
    theme: &Theme,
    color_file_names: bool,
) {
    let first = edit.viewport.scroll.min(edit.lines.len());
    let last = first
        .saturating_add(usize::from(area.height))
        .min(edit.lines.len());
    let items = edit.lines[first..last]
        .iter()
        .enumerate()
        .map(|(offset, line)| {
            edit_item(
                line,
                first + offset,
                edit.cursor_row,
                line_numbers,
                icon_mode,
                theme,
                color_file_names,
            )
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(items).style(Style::default().bg(theme.base.into())),
        area,
    );

    draw_cursor(
        frame,
        area,
        edit,
        line_numbers,
        icon_mode,
        theme,
        first..last,
    );
}

fn edit_item<'a>(
    line: &'a EditLine,
    index: usize,
    cursor_row: usize,
    line_numbers: LineNumberMode,
    icon_mode: IconMode,
    theme: &Theme,
    color_file_names: bool,
) -> ListItem<'a> {
    let icon_kind = line
        .original
        .as_ref()
        .map_or(EntryIconKind::New, |original| {
            if original.is_directory {
                EntryIconKind::Directory
            } else {
                EntryIconKind::File
            }
        });
    let icon = edit_line_icon(line, icon_mode, theme);

    let mut spans = vec![
        line_number_span(line_numbers, index, cursor_row, theme),
        Span::styled(
            icon.glyph,
            Style::default().fg(
                if line.original.as_ref().is_some_and(|entry| entry.is_hidden) {
                    theme.hidden
                } else {
                    icon.themed_display_color(icon_kind, theme)
                }
                .into(),
            ),
        ),
        Span::raw(icon.spacer()),
        Span::styled(
            line.display_text(),
            Style::default().fg(
                if line.original.as_ref().is_some_and(|entry| entry.is_hidden) {
                    theme.hidden
                } else {
                    icon.file_label_color(&line.text, icon_kind, theme, color_file_names)
                }
                .into(),
            ),
        ),
    ];
    if index == cursor_row && theme.selected_text != theme.text {
        for span in &mut spans {
            span.style = span.style.fg(theme.selected_text.into());
        }
    }

    ListItem::new(Line::from(spans)).style(if index == cursor_row {
        Style::default()
            .fg(theme.selected_text.into())
            .bg(theme.selected.into())
    } else {
        Style::default().fg(theme.text.into())
    })
}

fn draw_cursor(
    frame: &mut Frame<'_>,
    area: Rect,
    edit: &DirectoryEditState,
    line_numbers: LineNumberMode,
    icon_mode: IconMode,
    theme: &Theme,
    visible: Range<usize>,
) {
    if !visible.contains(&edit.cursor_row) || area.width == 0 {
        return;
    }

    let icon = edit_line_icon(&edit.lines[edit.cursor_row], icon_mode, theme);
    let prefix = format!(
        "{}{}{}",
        line_number(line_numbers, edit.cursor_row, edit.cursor_row),
        icon.glyph,
        icon.spacer()
    );
    let before = edit.lines[edit.cursor_row]
        .text
        .chars()
        .take(edit.cursor_column)
        .collect::<String>();
    let column = Line::from(format!("{prefix}{before}"))
        .width()
        .min(usize::from(area.width.saturating_sub(1))) as u16;

    frame.set_cursor_position((
        area.x + column,
        area.y + (edit.cursor_row - visible.start) as u16,
    ));
}

fn edit_line_icon(line: &EditLine, mode: IconMode, theme: &Theme) -> crate::icons::ResolvedIcon {
    line.original.as_ref().map_or_else(
        || resolve_for_theme(mode, line.text.as_str(), EntryIconKind::New, theme),
        |original| {
            resolve_for_theme(
                mode,
                original.name.as_str(),
                if original.is_directory {
                    EntryIconKind::Directory
                } else {
                    EntryIconKind::File
                },
                theme,
            )
        },
    )
}
