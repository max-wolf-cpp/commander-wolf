use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::finder::{FinderEntryKind, FinderMode, FinderResult, FinderState};
use crate::icons::{EntryIconKind, resolve_for_theme};
use crate::theme::Theme;
use crate::workspace::PanelStyle;

use super::layout::{PreviewLayout, preview_layout as resolve_preview_layout};

const FINDER_MEDIA_PERCENT: u16 = 70;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinderLayout {
    pub modal: Rect,
    pub results: Rect,
    pub results_inner: Rect,
    pub input: Rect,
    pub input_inner: Rect,
    pub preview: PreviewLayout,
}

pub fn modal_area(area: Rect) -> Rect {
    let width = if area.width >= 20 {
        (area.width.saturating_mul(4) / 5).max(20)
    } else {
        area.width
    }
    .min(area.width);
    let height = if area.height >= 5 {
        (area.height.saturating_mul(4) / 5).max(5)
    } else {
        area.height
    }
    .min(area.height);

    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

pub fn resolve_layout(area: Rect) -> FinderLayout {
    let modal = modal_area(area);
    let [left, right] =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas(modal);
    let [results, input] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(left);

    FinderLayout {
        modal,
        results,
        results_inner: results.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }),
        input,
        input_inner: input.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }),
        preview: resolve_preview_layout(right, FINDER_MEDIA_PERCENT, PanelStyle::Framed),
    }
}

pub fn visible_rows(area: Rect) -> usize {
    usize::from(resolve_layout(area).results_inner.height)
}

pub fn preview_layout(area: Rect) -> PreviewLayout {
    resolve_layout(area).preview
}

pub fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let layout = resolve_layout(frame.area());
    let background = Style::default().bg(theme.float_background.into());
    super::floating::clear(frame, layout.modal, theme.float_background);
    let title_style =
        Style::default()
            .fg(theme.active_title.into())
            .add_modifier(if theme.title_bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

    draw_results(frame, layout, state, theme, background, title_style);
    draw_input(frame, layout, state, theme, background, title_style);

    super::preview::draw_with_chrome(
        frame,
        layout.preview,
        state,
        theme,
        super::preview::PreviewChrome {
            title: "Preview",
            panel_style: PanelStyle::Framed,
            background_color: theme.float_background,
            border_color: theme.active_border,
            title_color: theme.active_title,
        },
    );
}

fn draw_results(
    frame: &mut Frame<'_>,
    layout: FinderLayout,
    state: &AppState,
    theme: &Theme,
    background: Style,
    title_style: Style,
) {
    super::chrome::draw_titled_block(
        frame,
        layout.results,
        Block::default()
            .style(background)
            .borders(Borders::ALL)
            .border_set(theme.border_set())
            .border_style(Style::default().fg(theme.active_border.into())),
        "Results",
        title_style,
        background,
    );

    let first = state.finder.viewport.scroll.min(state.finder.results.len());
    let last = first
        .saturating_add(usize::from(layout.results_inner.height))
        .min(state.finder.results.len());
    let mut items = state.finder.results[first..last]
        .iter()
        .enumerate()
        .map(|(visible, result)| {
            result_item(
                result,
                first + visible == state.finder.selected,
                state,
                theme,
                background,
            )
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        let text = state
            .finder
            .error
            .as_deref()
            .unwrap_or(match state.finder.mode {
                FinderMode::Searching => "Searching…",
                _ => "No results",
            });
        items.push(
            ListItem::new(text).style(background.fg(if state.finder.error.is_some() {
                theme.error.into()
            } else {
                theme.inactive_title.into()
            })),
        );
    }

    frame.render_widget(List::new(items).style(background), layout.results_inner);
}

fn result_item<'a>(
    result: &'a FinderResult,
    selected: bool,
    state: &AppState,
    theme: &Theme,
    background: Style,
) -> ListItem<'a> {
    let icon_kind = match result.kind {
        FinderEntryKind::File => EntryIconKind::File,
        FinderEntryKind::Directory => EntryIconKind::Directory,
    };
    let icon = resolve_for_theme(
        state.workspace.icon_mode,
        result.relative_path.as_str(),
        icon_kind,
        theme,
    );
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            icon.glyph,
            Style::default().fg(if result.is_hidden {
                theme.hidden
            } else {
                icon.themed_display_color(icon_kind, theme)
            }
            .into()),
        ),
        Span::raw(icon.spacer()),
        Span::styled(
            state
                .workspace
                .icon_mode
                .label(&result.relative_path, icon_kind),
            Style::default().fg(if result.is_hidden {
                theme.hidden
            } else {
                icon.file_label_color(
                    &result.relative_path,
                    icon_kind,
                    theme,
                    state.workspace.color_file_names,
                )
            }
            .into()),
        ),
    ];
    if selected && theme.selected_text != theme.text {
        for span in &mut spans {
            span.style = span.style.fg(theme.selected_text.into());
        }
    }

    ListItem::new(Line::from(spans)).style(if selected {
        Style::default()
            .fg(theme.selected_text.into())
            .bg(theme.selected.into())
    } else {
        background.fg(theme.text.into())
    })
}

fn draw_input(
    frame: &mut Frame<'_>,
    layout: FinderLayout,
    state: &AppState,
    theme: &Theme,
    background: Style,
    title_style: Style,
) {
    let status = finder_status(&state.finder);
    let prefix = format!("> {}", state.finder.pattern);
    let right = if state.pending_keys.is_empty() {
        status
    } else {
        format!("{status}  {}", state.pending_keys)
    };

    super::chrome::draw_titled_block(
        frame,
        layout.input,
        Block::default()
            .style(background)
            .borders(Borders::ALL)
            .border_set(theme.border_set())
            .border_style(Style::default().fg(theme.active_border.into())),
        "Find Files",
        title_style,
        background,
    );
    let available = usize::from(layout.input_inner.width);
    let gap = available.saturating_sub(prefix.chars().count() + right.chars().count());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix.clone(), background.fg(theme.text.into())),
            Span::raw(" ".repeat(gap)),
            Span::styled(right, background.fg(theme.inactive_title.into())),
        ]))
        .style(background),
        layout.input_inner,
    );

    if state.finder.mode == FinderMode::Insert && layout.input_inner.width > 0 {
        let cursor = prefix
            .chars()
            .count()
            .min(usize::from(layout.input_inner.width.saturating_sub(1)))
            as u16;
        frame.set_cursor_position((layout.input_inner.x + cursor, layout.input_inner.y));
    }
}

fn finder_status(finder: &FinderState) -> String {
    if let Some(error) = &finder.error {
        return format!("Error: {error}");
    }

    match finder.mode {
        FinderMode::Insert => "INSERT".to_owned(),
        FinderMode::Searching => format!("Searching… {}", finder.results.len()),
        FinderMode::Normal => {
            let mut value = format!(
                "{} / {}",
                if finder.results.is_empty() {
                    0
                } else {
                    finder.selected + 1
                },
                finder.results.len(),
            );
            if finder.truncated {
                value.push_str(" · truncated");
            }
            if finder.skipped > 0 {
                value.push_str(&format!(" · skipped {}", finder.skipped));
            }

            value
        }
        FinderMode::Closed => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_is_bounded_and_reports_consistent_result_rows() {
        for area in [Rect::new(0, 0, 10, 3), Rect::new(0, 0, 100, 40)] {
            let modal = modal_area(area);
            assert!(modal.width <= area.width);
            assert!(modal.height <= area.height);
            assert_eq!(
                visible_rows(area),
                usize::from(resolve_layout(area).results_inner.height)
            );
        }
        assert_eq!(
            modal_area(Rect::new(0, 0, 200, 100)),
            Rect::new(20, 10, 160, 80)
        );
    }

    #[test]
    fn finder_uses_three_independent_titled_regions() {
        let layout = resolve_layout(Rect::new(0, 0, 100, 40));
        assert_eq!(layout.results.x, layout.input.x);
        assert_eq!(layout.results.width, layout.input.width);
        assert_eq!(layout.results.bottom(), layout.input.top());
        assert_eq!(layout.input.bottom(), layout.modal.bottom());
        assert_eq!(layout.preview.panel.top(), layout.modal.top());
        assert_eq!(layout.preview.panel.bottom(), layout.modal.bottom());
        assert_eq!(layout.results.right(), layout.preview.panel.left());
        let content_height = layout
            .preview
            .inner
            .height
            .saturating_sub(layout.preview.separator.height);
        assert_eq!(
            layout.preview.media.height,
            (u32::from(content_height) * 70 / 100) as u16
        );
    }
}
