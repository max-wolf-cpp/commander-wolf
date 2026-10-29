use super::documents::start_document_request;
use super::playback::stop_video;
use crate::app::{Action, AppState, Effect};
use crate::preview::{DocumentFormat, LoadState};
use crate::workspace::PanelId;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::TogglePreview => {
            if state.workspace.preview_visible {
                let mut effects = stop_video(state);
                effects.extend(clear_preview(state));
                state.workspace.preview_visible = false;
                state.preview.clear();
                effects
            } else {
                state.workspace.preview_visible = true;
                prepare_selected(state)
            }
        }
        _ => unreachable!("dispatcher routed an unsupported action to preview"),
    }
}

pub(super) fn clear_preview(state: &AppState) -> Vec<Effect> {
    let mut effects = Vec::new();
    if state.finder.is_open() || state.workspace.preview_panel().is_some() {
        effects.push(Effect::ClearMediaSurface);
    }

    effects.push(Effect::CancelDocumentPreview);
    effects
}

pub(super) fn prepare_selected(state: &mut AppState) -> Vec<Effect> {
    if state.finder.is_open() || state.viewer.is_open() {
        return Vec::new();
    }

    let Some(panel) = state.workspace.preview_panel() else {
        return Vec::new();
    };

    let Some(entry) = state.selected_entry().cloned() else {
        state.preview.clear();
        return vec![Effect::ClearMediaSurface];
    };

    prepare_preview_path(state, panel, entry.path, entry.name, entry.is_directory)
}

pub(super) fn prepare_finder_selected(state: &mut AppState) -> Vec<Effect> {
    let Some(result) = state.finder.selected_result().cloned() else {
        state.preview.clear();
        return vec![Effect::ClearMediaSurface, Effect::CancelDocumentPreview];
    };

    let title = result
        .absolute_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| result.relative_path.clone());
    prepare_preview_path(
        state,
        state.finder.panel,
        result.absolute_path,
        title,
        result.kind == crate::finder::FinderEntryKind::Directory,
    )
}

fn prepare_preview_path(
    state: &mut AppState,
    panel: PanelId,
    path: std::path::PathBuf,
    title: String,
    is_directory: bool,
) -> Vec<Effect> {
    let generation = state.preview.begin(path.clone(), title);
    state.preview.metadata_viewport.visible_rows =
        usize::from(preview_metadata_area(state, panel).height);

    let mut effects = vec![
        Effect::ClearMediaSurface,
        Effect::CancelDocumentPreview,
        Effect::LoadPreviewMetadata {
            generation,
            path: path.clone(),
        },
    ];

    if is_directory {
        state.preview.media = LoadState::Idle;
        state.preview.directory = LoadState::Loading;
        effects.push(Effect::LoadPreviewDirectory {
            generation,
            path,
            show_hidden: state.workspace.show_hidden,
        });
    } else if state.preview_options.documents.enabled
        && let Some(format) = DocumentFormat::from_path(&path)
    {
        state.preview.media = LoadState::Idle;
        state.preview.document = LoadState::Loading;
        effects.push(start_document_request(state, panel, path, format, 0));
    } else {
        effects.push(Effect::PreparePreviewMedia {
            generation,
            path,
            panel,
            region: preview_media_region(state, panel),
        });
    }

    effects
}

pub(super) fn restore_preview_media(state: &mut AppState) -> Vec<Effect> {
    let (Some(panel), Some(path)) = (active_preview_panel(state), state.preview.path.clone())
    else {
        return Vec::new();
    };

    state.preview.media = LoadState::Loading;

    vec![Effect::PreparePreviewMedia {
        generation: state.preview.generation,
        path,
        panel,
        region: preview_media_region(state, panel),
    }]
}

pub(super) fn preview_media_region(state: &AppState, panel: PanelId) -> fox_media_term::Region {
    let rect = if state.finder.is_open() {
        crate::ui::finder_preview_layout(state.terminal_size.area()).media
    } else {
        crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            Some(panel),
            state.preview_options.media_percent,
            state.status_line_options.style,
            state.workspace.panel_style,
        )
        .preview
        .map_or_else(ratatui::layout::Rect::default, |layout| layout.media)
    };

    fox_media_term::Region {
        column: i32::from(rect.x),
        row: i32::from(rect.y),
        columns: i32::from(rect.width),
        rows: i32::from(rect.height),
    }
}

pub(super) fn active_preview_panel(state: &AppState) -> Option<PanelId> {
    if state.finder.is_open() && state.finder.selected_result().is_some() {
        Some(state.finder.panel)
    } else if !state.finder.is_open() && !state.directory_tree.open {
        state.workspace.preview_panel()
    } else {
        None
    }
}

fn preview_metadata_area(state: &AppState, panel: PanelId) -> ratatui::layout::Rect {
    let area = if state.finder.is_open() {
        crate::ui::finder_preview_layout(state.terminal_size.area()).metadata
    } else {
        crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            Some(panel),
            state.preview_options.media_percent,
            state.status_line_options.style,
            state.workspace.panel_style,
        )
        .preview
        .map_or_else(ratatui::layout::Rect::default, |layout| layout.metadata)
    };

    area.inner(ratatui::layout::Margin::new(1, 0))
}

pub(super) fn metadata_content_rows(state: &AppState) -> usize {
    let Some(panel) = active_preview_panel(state) else {
        return state.preview.metadata_rows().len();
    };

    let area = preview_metadata_area(state, panel);
    state.preview.metadata_layout(area.width).rows.len()
}

pub(super) fn clamp_metadata_viewport(state: &mut AppState) {
    let rows = metadata_content_rows(state);
    state
        .preview
        .metadata_viewport
        .set_visible_rows(state.preview.metadata_viewport.visible_rows, rows);
}

pub(super) fn resize_preview(state: &mut AppState) {
    let Some(panel) = active_preview_panel(state) else {
        return;
    };

    let area = preview_metadata_area(state, panel);
    let visible_rows = usize::from(area.height);
    let content_rows = state.preview.metadata_layout(area.width).rows.len();
    state
        .preview
        .metadata_viewport
        .set_visible_rows(visible_rows, content_rows);
}
