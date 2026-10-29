use ratatui::style::Color;

use super::colors::rgb24;
use super::{FileColorPolicy, Rgb, Theme};
use crate::terminal_appearance::{Scheme, TerminalAppearance};

pub fn derive_default_theme(appearance: TerminalAppearance) -> Theme {
    let light = appearance.scheme == Scheme::Light;
    let background = appearance.background.unwrap_or(if light {
        rgb24(0xFFFFFF)
    } else {
        rgb24(0x000000)
    });

    let mut theme = Theme::emergency_monochrome();
    theme.base = Color::Reset;
    theme.float_background = Color::Reset;
    theme.preview_background = Color::Reset;
    theme.status_background = Color::Reset;
    theme.compact_status_background = Color::Reset;
    theme.command_background = Color::Reset;
    theme.text = Color::Reset;
    theme.active_border = Color::Indexed(4);
    theme.inactive_border = Color::Reset;
    theme.active_title = Color::Indexed(4);
    theme.inactive_title = Color::Reset;

    apply_panel_colors(&mut theme, appearance, background, light);
    let fallback_selection = apply_selection_colors(&mut theme, appearance, background, light);
    apply_search_colors(&mut theme, appearance, background, light);

    theme.error = Color::Indexed(1);
    theme.dialog_info = Color::Indexed(6);
    theme.dialog_warning = Color::Indexed(3);
    theme.dialog_error = Color::Indexed(1);
    theme.directory_edit_create = Color::Indexed(2);
    theme.directory_edit_rename = Color::Indexed(3);
    theme.directory_edit_delete = Color::Indexed(1);

    theme.preview_separator = theme.hidden;
    theme.preview_metadata_label = Color::Indexed(6);
    theme.preview_metadata_value = Color::Reset;
    theme.preview_unavailable = Color::Reset;
    theme.preview_visualizer_low = Color::Indexed(6);
    theme.preview_visualizer_high = Color::Indexed(2);

    apply_status_colors(&mut theme, appearance, fallback_selection);

    theme.title_bold = true;
    theme.status_bold = true;
    theme.dim_secondary = true;
    theme.light_file_icons = light;

    if let Some(colors) = appearance.palette {
        theme.use_nearest_file_palette(colors, background, true);
    }
    theme.file_color_policy = FileColorPolicy::OriginalDeviconsRgb;

    theme
}

fn apply_panel_colors(
    theme: &mut Theme,
    appearance: TerminalAppearance,
    background: Rgb,
    light: bool,
) {
    theme.borderless_active_title_background = theme.active_border;
    theme.borderless_inactive_title_background = theme.inactive_border;
    theme.borderless_title_text = appearance.palette.map_or(Color::Reset, |colors| {
        Color::Indexed(crate::palette_colors::readable_palette_text(colors[4], &colors) as u8)
    });
    theme.borderless_inactive_title_text = Some(Color::Reset);
    theme.borderless_preview_title_background = theme.base;
    theme.preview_title_text = Some(theme.text);
    theme.borderless_title_left = "".into();
    theme.borderless_title_right = "".into();

    theme.directory = Color::Indexed(4);
    let neutral = if light {
        rgb24(0x000000)
    } else {
        rgb24(0xFFFFFF)
    };
    theme.hidden = blend(background, neutral, if light { 45 } else { 40 });
    theme.unknown_file_icon = Color::Reset;
    theme.line_number = theme.hidden;
    theme.current_line_number = Color::Indexed(4);
}

fn apply_selection_colors(
    theme: &mut Theme,
    appearance: TerminalAppearance,
    background: Rgb,
    light: bool,
) -> Color {
    let fallback_selection = blend(background, rgb24(0x0064FF), if light { 25 } else { 45 });
    theme.selected = if appearance.palette.is_some() {
        Color::Indexed(4)
    } else {
        fallback_selection
    };
    theme.selected_text = appearance.palette.map_or_else(
        || contrast_text(theme.selected),
        |colors| {
            Color::Indexed(crate::palette_colors::readable_palette_text(colors[4], &colors) as u8)
        },
    );

    let neutral = if light {
        rgb24(0x000000)
    } else {
        rgb24(0xFFFFFF)
    };
    let inactive_selection = blend(background, neutral, if light { 15 } else { 30 });
    theme.inactive_selected = Some(inactive_selection);
    theme.inactive_selected_text = Some(contrast_text(inactive_selection));

    theme.visual_selection_background = blend(background, neutral, if light { 25 } else { 45 });
    theme.visual_selection_text =
        contrast_text_from_palette(theme.visual_selection_background, appearance.palette);

    fallback_selection
}

fn apply_search_colors(
    theme: &mut Theme,
    appearance: TerminalAppearance,
    background: Rgb,
    light: bool,
) {
    let search_color = appearance
        .palette
        .map_or(rgb24(0xFFB900), |colors| colors[3]);
    theme.search_match_background = blend(background, search_color, if light { 50 } else { 55 });
    theme.search_match_text =
        contrast_text_from_palette(theme.search_match_background, appearance.palette);

    let current_search_color = appearance
        .palette
        .map_or(rgb24(0x00B4BE), |colors| colors[6]);
    theme.search_current_match_background = blend(
        background,
        current_search_color,
        if light { 45 } else { 55 },
    );
    theme.search_current_match_text =
        contrast_text_from_palette(theme.search_current_match_background, appearance.palette);
}

fn apply_status_colors(
    theme: &mut Theme,
    appearance: TerminalAppearance,
    fallback_selection: Color,
) {
    theme.status_border = Color::Reset;
    theme.framed_footer_border = Color::Reset;
    theme.framed_footer_border_follows_mode = false;

    theme.mode_normal = Color::Indexed(4);
    theme.mode_insert = Color::Indexed(2);
    theme.mode_search = Color::Indexed(6);
    theme.mode_visual = Color::Indexed(5);
    theme.mode_command = Color::Indexed(3);

    theme.compact_mode_text = contrast_text(fallback_selection);
    theme.compact_mode_background = if appearance.palette.is_some() {
        None
    } else {
        Some(fallback_selection)
    };
    theme.terminal_palette = appearance.palette;
    theme.auto_compact_mode_text = appearance.palette.is_some();
    theme.compact_mode_separator_left = "".into();
    theme.compact_mode_separator_right = "".into();
}

fn blend(background: Rgb, foreground: Rgb, foreground_percent: u16) -> Color {
    let component = |base: u8, accent: u8| {
        ((u16::from(base) * (100 - foreground_percent)
            + u16::from(accent) * foreground_percent
            + 50)
            / 100) as u8
    };

    Color::Rgb(
        component(background.red, foreground.red),
        component(background.green, foreground.green),
        component(background.blue, foreground.blue),
    )
}

fn contrast_text(surface: Color) -> Color {
    let Color::Rgb(red, green, blue) = surface else {
        return Color::Reset;
    };

    if crate::palette_colors::is_light_background(Rgb { red, green, blue }) {
        Color::Rgb(0, 0, 0)
    } else {
        Color::Rgb(255, 255, 255)
    }
}

fn contrast_text_from_palette(surface: Color, palette: Option<[Rgb; 16]>) -> Color {
    if let (Some(colors), Color::Rgb(red, green, blue)) = (palette, surface) {
        let background = Rgb { red, green, blue };
        Color::Indexed(crate::palette_colors::readable_palette_text(background, &colors) as u8)
    } else {
        contrast_text(surface)
    }
}
