use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use crate::icons::{EntryIconKind, IconMode, resolve_for_theme};
use crate::search::match_ranges;
use crate::theme::Theme;
use crate::workspace::{DirectoryEntry, LineNumberMode, PanelState};

use super::PanelOverlay;

pub(super) fn entry_item<'a>(
    entry: &'a DirectoryEntry,
    index: usize,
    panel: &PanelState,
    line_numbers: LineNumberMode,
    active: bool,
    overlay: &PanelOverlay<'_>,
    theme: &Theme,
) -> ListItem<'a> {
    let icon_kind = entry_icon_kind(entry);
    let icon = resolve_for_theme(overlay.icon_mode, entry.name.as_str(), icon_kind, theme);
    let marked = panel.is_marked(&entry.path);

    let mut spans = vec![
        line_number_span(line_numbers, index, panel.selected, theme),
        Span::styled(
            if marked {
                if overlay.icon_mode == IconMode::None {
                    "*"
                } else {
                    "●"
                }
            } else {
                " "
            },
            Style::default().fg(theme.active_title.into()),
        ),
        Span::raw(" "),
        Span::styled(
            icon.glyph,
            Style::default().fg(if entry.is_hidden {
                theme.hidden
            } else {
                icon.themed_display_color(icon_kind, theme)
            }
            .into()),
        ),
        Span::raw(icon.spacer()),
    ];
    spans.extend(name_spans(
        entry.name.as_str(),
        overlay.search.1.highlighted_query(overlay.search.0),
        overlay.search.1.current_match == Some(index)
            && overlay.search.1.highlight_panel == Some(overlay.search.0),
        theme,
        if entry.is_hidden {
            theme.hidden
        } else {
            icon.file_label_color(&entry.name, icon_kind, theme, overlay.color_file_names)
        },
    ));
    spans.push(Span::styled(
        overlay.icon_mode.directory_suffix(&entry.name, icon_kind),
        Style::default().fg(if entry.is_hidden {
            theme.hidden
        } else {
            theme.directory
        }
        .into()),
    ));

    let selected = active && index == panel.selected;
    let inactive_selected = !active && index == panel.selected && theme.inactive_selected.is_some();
    let visually_selected = active
        && !entry.is_parent
        && overlay.visual_anchor.is_some_and(|anchor| {
            (anchor.min(panel.selected)..=anchor.max(panel.selected)).contains(&index)
        });
    apply_selection_colors(
        &mut spans,
        selected,
        inactive_selected,
        visually_selected,
        theme,
    );

    ListItem::new(Line::from(spans)).style(selection_style(
        selected,
        inactive_selected,
        visually_selected,
        theme,
    ))
}

fn apply_selection_colors(
    spans: &mut [Span<'_>],
    selected: bool,
    inactive_selected: bool,
    visually_selected: bool,
    theme: &Theme,
) {
    if visually_selected {
        // Visual range contrast takes precedence over devicons and search highlights.
        for span in spans {
            span.style = span
                .style
                .fg(theme.visual_selection_text.into())
                .bg(theme.visual_selection_background.into());
        }
    } else if selected && theme.selected_text != theme.text {
        for span in spans {
            if span.style.bg.is_none() {
                span.style = span.style.fg(theme.selected_text.into());
            }
        }
    } else if inactive_selected {
        for span in spans {
            if span.style.bg.is_none() {
                span.style = span
                    .style
                    .fg(theme.inactive_selected_text.unwrap_or(theme.text).into());
            }
        }
    }
}

fn selection_style(
    selected: bool,
    inactive_selected: bool,
    visually_selected: bool,
    theme: &Theme,
) -> Style {
    if visually_selected {
        Style::default()
            .fg(theme.visual_selection_text.into())
            .bg(theme.visual_selection_background.into())
    } else if selected {
        Style::default()
            .fg(theme.selected_text.into())
            .bg(theme.selected.into())
    } else if inactive_selected {
        Style::default()
            .fg(theme.inactive_selected_text.unwrap_or(theme.text).into())
            .bg(theme.inactive_selected.unwrap().into())
    } else {
        Style::default().fg(theme.text.into())
    }
}

fn entry_icon_kind(entry: &DirectoryEntry) -> EntryIconKind {
    if entry.is_parent {
        EntryIconKind::Parent
    } else if entry.is_directory {
        EntryIconKind::Directory
    } else {
        EntryIconKind::File
    }
}

pub(super) fn name_spans<'a>(
    name: &'a str,
    query: Option<&str>,
    current: bool,
    theme: &Theme,
    base_color: Color,
) -> Vec<Span<'a>> {
    let base_style = Style::default().fg(base_color.into());
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return vec![Span::styled(name, base_style)];
    };

    let ranges = match_ranges(name, query);
    if ranges.is_empty() {
        return vec![Span::styled(name, base_style)];
    }

    let match_style = if current {
        Style::default()
            .fg(theme.search_current_match_text.into())
            .bg(theme.search_current_match_background.into())
    } else {
        Style::default()
            .fg(theme.search_match_text.into())
            .bg(theme.search_match_background.into())
    };
    let mut spans = Vec::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            spans.push(Span::styled(&name[cursor..start], base_style));
        }
        spans.push(Span::styled(&name[start..end], match_style));
        cursor = end;
    }
    if cursor < name.len() {
        spans.push(Span::styled(&name[cursor..], base_style));
    }

    spans
}

pub(super) fn line_number(mode: LineNumberMode, index: usize, selected: usize) -> String {
    if mode == LineNumberMode::Off {
        return String::new();
    }

    let value = if mode == LineNumberMode::Relative && index != selected {
        index.abs_diff(selected)
    } else {
        index + 1
    };

    if index == selected {
        format!("{value:<4} ")
    } else {
        format!("{value:>4} ")
    }
}

pub(super) fn line_number_span(
    mode: LineNumberMode,
    index: usize,
    selected: usize,
    theme: &Theme,
) -> Span<'static> {
    Span::styled(
        line_number(mode, index, selected),
        Style::default().fg(if index == selected {
            theme.current_line_number.into()
        } else {
            theme.line_number.into()
        }),
    )
}
