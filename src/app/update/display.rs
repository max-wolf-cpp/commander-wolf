use super::documents::start_document_request;
use super::playback::stop_video;
use super::preview::{
    prepare_finder_selected, prepare_selected, resize_preview, restore_preview_media,
};
use crate::app::{Action, AppState, Effect, Notification};
use crate::preview::{DocumentFormat, LoadState, PlaybackState};
use crate::status_line::UiStyle;
use crate::workspace::PanelStyle;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::SetStylePreset(preset) => set_ui_styles(
            state,
            preset.footer_style(),
            preset.panel_style(),
            format!("Style '{}' active", preset.name()),
        ),
        Action::SetFooterStyle(style) => set_ui_styles(
            state,
            style,
            state.workspace.panel_style,
            format!("Footer style '{}' active", style.name()),
        ),
        Action::SetPanelStyle(style) => set_ui_styles(
            state,
            state.status_line_options.style,
            style,
            format!("Panel style '{}' active", style.name()),
        ),
        Action::SetSeparatorStyle(style) => {
            state.separator_style_override = Some(style);
            let name = match style {
                crate::theme::SeparatorStyle::Thin => "plain",
                crate::theme::SeparatorStyle::Rounded => "rounded",
                crate::theme::SeparatorStyle::Double => "double",
                crate::theme::SeparatorStyle::Thick => "thick",
                crate::theme::SeparatorStyle::Simple => "simple",
                crate::theme::SeparatorStyle::Dashed => "dashed",
                crate::theme::SeparatorStyle::Off => "off",
            };
            state.notification = Some(Notification::info(format!("Separators: {name}")));
            Vec::new()
        }
        Action::SetBorderStyle(style) => {
            state.border_style_override = Some(style);
            state.notification = Some(Notification::info(format!("Border: {}", style.name())));
            Vec::new()
        }
        Action::SetIconMode(mode) => {
            state.workspace.icon_mode = mode;
            let name = match mode {
                crate::icons::IconMode::NerdFont => "nerd_font",
                crate::icons::IconMode::Unicode => "unicode",
                crate::icons::IconMode::None => "none",
            };
            state.notification = Some(Notification::info(format!("Icons: {name}")));
            Vec::new()
        }
        Action::SetNameColors(enabled) => {
            state.workspace.color_file_names = enabled;
            state.notification = Some(Notification::info(format!(
                "Name colors: {}",
                if enabled { "on" } else { "off" }
            )));
            Vec::new()
        }
        Action::SetDynamicFileColorPolicy(policy) => {
            vec![Effect::SetDynamicFileColorPolicy(policy)]
        }
        Action::UseTheme(name) => vec![Effect::UseTheme(name)],
        Action::UsePalette(name) => vec![Effect::UsePalette(name)],
        Action::UseSyntax(name) => vec![Effect::UseSyntax(name)],
        Action::UseDisplayStyle(name) => vec![Effect::UseDisplayStyle(name)],
        Action::ApplyDisplayStyle(style) => {
            state.separator_style_override = None;
            state.border_style_override = None;
            state.workspace.icon_mode = style.icons;
            state.workspace.color_file_names = style.color_file_names;
            set_ui_styles(state, style.footer, style.panels, "Style active".into())
        }
        Action::Resize {
            size,
            panel_rows,
            finder_rows,
            tree_rows,
        } => resize(state, size, panel_rows, finder_rows, tree_rows),
        _ => unreachable!("dispatcher routed an unsupported action to display"),
    }
}

fn set_ui_styles(
    state: &mut AppState,
    footer_style: UiStyle,
    panel_style: PanelStyle,
    notification: String,
) -> Vec<Effect> {
    if state.status_line_options.style == footer_style && state.workspace.panel_style == panel_style
    {
        return Vec::new();
    }

    let mut effects = stop_video(state);
    state.status_line_options.style = footer_style;
    state.workspace.panel_style = panel_style;

    let panel_rows = usize::from(
        crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            state.workspace.preview_panel(),
            state.preview_options.media_percent,
            footer_style,
            panel_style,
        )
        .left_inner
        .height,
    );
    state.workspace.set_visible_rows(panel_rows);
    resize_preview(state);
    state.notification = Some(Notification::info(notification));

    if state.workspace.preview_visible {
        effects.extend(prepare_selected(state));
    }

    effects
}

fn resize(
    state: &mut AppState,
    size: crate::terminal::TerminalSize,
    panel_rows: usize,
    finder_rows: usize,
    tree_rows: usize,
) -> Vec<Effect> {
    state.terminal_size = size;
    if let Some(help) = &mut state.help {
        help.clamp_scroll(crate::ui::help_visible_rows(size.area()));
    }

    state.workspace.set_visible_rows(panel_rows);
    state.directory_edit.set_visible_rows(panel_rows);
    state
        .finder
        .set_visible_rows(finder_rows, state.finder_options.scroll_offset);
    state.directory_tree.set_visible_rows(tree_rows);

    state.preview.playback = PlaybackState::Stopped;
    let mut effects = vec![Effect::StopPreviewPlayback, Effect::ClearMediaSurface];

    if state.viewer.is_open() {
        effects.extend(resize_viewer(state, size));
    } else if state.finder.is_open() {
        resize_preview(state);
        effects.extend(prepare_finder_selected(state));
    } else if !state.directory_tree.open {
        resize_preview(state);
        effects.extend(resize_workspace_preview(state));
    }

    effects
}

fn resize_viewer(state: &mut AppState, size: crate::terminal::TerminalSize) -> Vec<Effect> {
    let Some(path) = state.viewer.path.clone() else {
        return Vec::new();
    };

    let title = state.viewer.title.clone();
    let generation = state.viewer.open(path.clone(), title.clone());

    vec![Effect::ProbeViewer {
        generation,
        path,
        title,
        size,
        maximum_percent: state.preview_options.viewer_max_percent,
    }]
}

fn resize_workspace_preview(state: &mut AppState) -> Vec<Effect> {
    let document_page = match &state.preview.document {
        LoadState::Ready(document) => Some(document.current_page),
        LoadState::Loading => Some(state.preview.document_page),
        LoadState::Idle | LoadState::Unavailable(_) => None,
    };
    if let Some(page) = document_page {
        let (Some(panel), Some(path), Some(format)) = (
            state.workspace.preview_panel(),
            state.preview.path.clone(),
            state
                .preview
                .path
                .as_deref()
                .and_then(DocumentFormat::from_path),
        ) else {
            return Vec::new();
        };

        return vec![
            Effect::CancelDocumentPreview,
            start_document_request(state, panel, path, format, page),
        ];
    }

    if matches!(state.preview.media, LoadState::Ready(_)) {
        return restore_preview_media(state);
    }

    Vec::new()
}
