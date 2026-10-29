use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::icons::{EntryIconKind, IconMode, resolve_for_theme};
use crate::theme::Theme;

pub(super) fn draw_archive(
    frame: &mut Frame<'_>,
    area: Rect,
    preview: &crate::preview::ArchivePreview,
    icon_mode: IconMode,
    theme: &Theme,
    background: Style,
    color_file_names: bool,
) {
    if preview.entries.is_empty() {
        frame.render_widget(
            Paragraph::new("Empty archive")
                .centered()
                .style(background.fg(theme.preview_unavailable.into())),
            area,
        );
        return;
    }

    let mut lines = preview
        .entries
        .iter()
        .take(usize::from(area.height))
        .map(|entry| {
            let icon_kind = if entry.is_directory {
                EntryIconKind::Directory
            } else {
                EntryIconKind::File
            };
            let icon = resolve_for_theme(icon_mode, entry.path.as_str(), icon_kind, theme);
            Line::from(vec![
                Span::styled(
                    icon.glyph,
                    Style::default().fg(icon.themed_display_color(icon_kind, theme).into()),
                ),
                Span::raw(icon.spacer()),
                Span::styled(
                    icon_mode.label(&entry.path, icon_kind),
                    Style::default().fg(icon
                        .file_label_color(&entry.path, icon_kind, theme, color_file_names)
                        .into()),
                ),
            ])
        })
        .collect::<Vec<_>>();
    if preview.truncated && lines.len() < usize::from(area.height) {
        lines.push(Line::styled(
            "… archive listing truncated …",
            Style::default().fg(theme.preview_unavailable.into()),
        ));
    }

    frame.render_widget(
        Paragraph::new(lines).style(background.fg(theme.text.into())),
        area,
    );
}

pub(super) fn draw_directory(
    frame: &mut Frame<'_>,
    area: Rect,
    preview: &crate::preview::DirectoryPreview,
    icon_mode: IconMode,
    theme: &Theme,
    background: Style,
    color_file_names: bool,
) {
    if preview.entries.is_empty() {
        frame.render_widget(
            Paragraph::new("Empty directory")
                .centered()
                .style(background.fg(theme.preview_unavailable.into())),
            area,
        );
        return;
    }

    let mut lines = preview
        .entries
        .iter()
        .take(usize::from(area.height))
        .map(|entry| {
            let icon_kind = if entry.is_directory {
                EntryIconKind::Directory
            } else {
                EntryIconKind::File
            };
            let icon = resolve_for_theme(icon_mode, entry.name.as_str(), icon_kind, theme);
            Line::from(vec![
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
                Span::styled(
                    icon_mode.label(&entry.name, icon_kind),
                    Style::default().fg(if entry.is_hidden {
                        theme.hidden
                    } else {
                        icon.file_label_color(&entry.name, icon_kind, theme, color_file_names)
                    }
                    .into()),
                ),
            ])
        })
        .collect::<Vec<_>>();
    if preview.truncated && lines.len() < usize::from(area.height) {
        lines.push(Line::styled(
            "… directory listing truncated …",
            Style::default().fg(theme.preview_unavailable.into()),
        ));
    }

    frame.render_widget(
        Paragraph::new(lines).style(background.fg(theme.text.into())),
        area,
    );
}
