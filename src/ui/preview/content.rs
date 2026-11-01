use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use crate::app::AppState;
use crate::preview::{DocumentPageContent, LoadState, PreviewKind};
use crate::theme::Theme;

use super::{audio, listing, text};

pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: &Theme,
    background: Style,
) {
    if state.preview.needs_surface_restore()
        || ((state.command_completion.is_some()
            || state.help.is_some()
            || state.shell_output.is_some()
            || state.operation_progress.is_some()
            || state.rename_prompt.is_some())
            && state.preview.media_owns_surface())
    {
        frame.render_widget(Block::default().style(background), area);
        return;
    }

    if state.preview.media_owns_surface() && area.width > 0 && area.height > 0 {
        super::super::mark_media_viewport_skipped(frame.buffer_mut(), area);
        return;
    }

    if matches!(
        state.preview.document,
        LoadState::Ready(crate::preview::DocumentPreview {
            content: DocumentPageContent::Raster {
                surface_ready: false,
                ..
            },
            ..
        })
    ) {
        draw_message(frame, area, "Loading document preview…", theme, background);
        return;
    }

    if matches!(
        state.preview.media,
        LoadState::Ready(crate::preview::MediaMetadata {
            kind: PreviewKind::Audio,
            ..
        })
    ) {
        audio::draw(frame, area, &state.preview.audio_levels, theme, background);
        return;
    }

    let text_area = area.inner(Margin::new(1, 0));
    if let LoadState::Ready(document) = &state.preview.document
        && let DocumentPageContent::Text(lines) = &document.content
    {
        frame.render_widget(
            Paragraph::new(
                lines
                    .iter()
                    .map(|line| Line::raw(line.as_str()))
                    .collect::<Vec<_>>(),
            )
            .style(background.fg(theme.text)),
            text_area,
        );
        return;
    }

    if let LoadState::Ready(preview) = &state.preview.archive {
        listing::draw_archive(
            frame,
            text_area,
            preview,
            state.workspace.icon_mode,
            theme,
            background,
            state.workspace.color_file_names,
        );
        return;
    }

    if let LoadState::Ready(preview) = &state.preview.directory {
        listing::draw_directory(
            frame,
            text_area,
            preview,
            state.workspace.icon_mode,
            theme,
            background,
            state.workspace.color_file_names,
        );
        return;
    }

    if let LoadState::Ready(preview) = &state.preview.text {
        text::draw(
            frame,
            text_area,
            preview,
            theme,
            &state.syntax_theme,
            state.syntax_enabled,
            background,
        );
        return;
    }

    if matches!(state.preview.media, LoadState::Loading) {
        frame.render_widget(Paragraph::new("").style(background), area);
        return;
    }

    draw_message(frame, area, fallback_message(state), theme, background);
}

fn fallback_message(state: &AppState) -> &str {
    if matches!(state.preview.document, LoadState::Loading) {
        return "Loading document preview…";
    }

    if matches!(state.preview.text, LoadState::Loading) {
        return "Loading text preview…";
    }

    if matches!(state.preview.directory, LoadState::Loading) {
        return "Loading directory preview…";
    }

    if matches!(state.preview.archive, LoadState::Loading) {
        return "Loading archive preview…";
    }

    match (
        &state.preview.archive,
        &state.preview.directory,
        &state.preview.text,
        &state.preview.media,
        &state.preview.document,
    ) {
        (LoadState::Unavailable(message), _, _, _, _)
        | (_, LoadState::Unavailable(message), _, _, _)
        | (_, _, LoadState::Unavailable(message), _, _)
        | (_, _, _, LoadState::Unavailable(message), _)
        | (_, _, _, _, LoadState::Unavailable(message)) => message.as_str(),
        _ => "No preview available",
    }
}

fn draw_message(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &str,
    theme: &Theme,
    background: Style,
) {
    frame.render_widget(
        Paragraph::new(message)
            .centered()
            .style(background.fg(theme.preview_unavailable)),
        area,
    );
}
