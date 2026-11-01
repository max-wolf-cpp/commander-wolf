use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::AppState;
use crate::directory_tree::TreeEntry;
use crate::icons::{EntryIconKind, IconMode, resolve_for_theme};
use crate::theme::Theme;

pub fn visible_rows(area: Rect) -> usize {
    usize::from(super::finder::modal_area(area).height.saturating_sub(2))
}

pub fn draw(frame: &mut Frame<'_>, state: &AppState, theme: &Theme) {
    let area = super::finder::modal_area(frame.area());
    let background = Style::default().bg(theme.float_background);
    super::floating::clear(frame, area, theme.float_background);
    let block = Block::default()
        .style(background)
        .borders(Borders::ALL)
        .border_set(theme.border_set())
        .border_style(Style::default().fg(theme.active_border));
    let title = format!("Directory Tree: {}", state.directory_tree.root.display());
    let inner = super::chrome::draw_titled_block(
        frame,
        area,
        block,
        &title,
        Style::default().fg(theme.active_title),
        background,
    );
    if inner.is_empty() {
        return;
    }

    let first = state
        .directory_tree
        .viewport
        .scroll
        .min(state.directory_tree.entries.len());
    let last = first
        .saturating_add(usize::from(inner.height))
        .min(state.directory_tree.entries.len());
    let mut items = state.directory_tree.entries[first..last]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            entry_item(
                entry,
                first + offset == state.directory_tree.selected,
                state,
                theme,
                background,
            )
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(
            ListItem::new(
                state
                    .directory_tree
                    .error
                    .as_deref()
                    .unwrap_or("Empty directory"),
            )
            .style(background.fg(if state.directory_tree.error.is_some() {
                theme.error
            } else {
                theme.inactive_title
            })),
        );
    }

    frame.render_widget(List::new(items).style(background), inner);
}

fn entry_item<'a>(
    entry: &'a TreeEntry,
    selected: bool,
    state: &AppState,
    theme: &Theme,
    background: Style,
) -> ListItem<'a> {
    let marker = entry_marker(entry, state.workspace.icon_mode);
    let icon_kind = entry_icon_kind(entry.is_directory, entry.expanded);
    let icon = resolve_for_theme(
        state.workspace.icon_mode,
        entry.name.as_str(),
        icon_kind,
        theme,
    );
    let label = if entry.depth == 0 {
        entry.path.display().to_string()
    } else {
        entry.name.clone()
    };
    let label = state
        .workspace
        .icon_mode
        .label(&label, icon_kind)
        .into_owned();

    let mut spans = vec![
        Span::raw("  ".repeat(entry.depth)),
        Span::raw(marker),
        Span::raw(if marker.is_empty() { "" } else { " " }),
        Span::styled(
            icon.glyph,
            Style::default().fg(if entry.is_hidden {
                theme.hidden
            } else {
                icon.themed_display_color(icon_kind, theme)
            }),
        ),
        Span::raw(icon.spacer()),
        Span::styled(
            label,
            Style::default().fg(if entry.is_hidden {
                theme.hidden
            } else {
                icon.file_label_color(
                    &entry.name,
                    icon_kind,
                    theme,
                    state.workspace.color_file_names,
                )
            }),
        ),
    ];
    if selected && theme.selected_text != theme.text {
        for span in &mut spans {
            span.style = span.style.fg(theme.selected_text);
        }
    }

    ListItem::new(Line::from(spans)).style(if selected {
        Style::default().fg(theme.selected_text).bg(theme.selected)
    } else {
        background.fg(theme.text)
    })
}

fn entry_marker(entry: &TreeEntry, icon_mode: IconMode) -> &'static str {
    if icon_mode == IconMode::Unicode && !entry.loading {
        ""
    } else if icon_mode == IconMode::None {
        if entry.loading {
            "."
        } else if entry.is_directory {
            if entry.expanded { "-" } else { "+" }
        } else {
            " "
        }
    } else if entry.loading {
        "…"
    } else if entry.is_directory && entry.expanded {
        "▾"
    } else if entry.is_directory {
        "▸"
    } else {
        " "
    }
}

fn entry_icon_kind(is_directory: bool, expanded: bool) -> EntryIconKind {
    match (is_directory, expanded) {
        (true, true) => EntryIconKind::DirectoryOpen,
        (true, false) => EntryIconKind::Directory,
        (false, _) => EntryIconKind::File,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::directory_tree::TreeEntry;
    use crate::icons::IconMode;
    use crate::terminal::TerminalSize;
    use crate::theme::Rgb;
    use crate::workspace::WorkspaceOptions;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn directory_icon_kind_tracks_expansion_state() {
        assert_eq!(entry_icon_kind(true, false), EntryIconKind::Directory);
        assert_eq!(entry_icon_kind(true, true), EntryIconKind::DirectoryOpen);
        assert_eq!(entry_icon_kind(false, true), EntryIconKind::File);
    }

    #[test]
    fn unicode_tree_renders_closed_and_open_folders_with_theme_color() {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            WorkspaceOptions {
                icon_mode: IconMode::Unicode,
                ..WorkspaceOptions::default()
            },
        );
        state.directory_tree.open = true;
        state.directory_tree.root = ".".into();
        state.directory_tree.entries = [("closed", false), ("open", true)]
            .map(|(name, expanded)| TreeEntry {
                is_hidden: false,
                path: name.into(),
                name: name.into(),
                depth: 1,
                is_directory: true,
                expanded,
                loading: false,
            })
            .to_vec();
        state.directory_tree.selected = 2;
        let mut theme = Theme::emergency_monochrome();
        theme.directory = Rgb::parse("#89B4FA").unwrap().into();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
        let cells = terminal.backend().buffer();
        for glyph in ["🗀", "🗁"] {
            let cell = cells
                .content()
                .iter()
                .find(|cell| cell.symbol() == glyph)
                .expect("folder glyph must be rendered");
            assert_eq!(cell.fg, theme.directory);
        }
    }
}
