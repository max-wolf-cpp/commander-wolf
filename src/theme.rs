use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::widgets::BorderType;

use crate::dialogs::DialogKind;
use crate::status_line::StatusMode;

mod borders;
mod colors;
mod defaults;
mod fields;
mod file_colors;

pub use borders::{BorderStyle, SeparatorStyle};
pub use colors::Rgb;
pub(crate) use colors::{FAR_PALETTE, format_theme_color, parse_theme_color};
pub use defaults::derive_default_theme;
pub use file_colors::{DynamicFileColorPolicy, FileColorPolicy, IconVariant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Theme {
    pub base: Color,
    pub float_background: Color,
    pub active_border: Color,
    pub inactive_border: Color,
    pub active_title: Color,
    pub inactive_title: Color,
    pub framed_active_title_background: Option<Color>,
    pub framed_active_title_text: Option<Color>,
    pub borderless_active_title_background: Color,
    pub borderless_inactive_title_background: Color,
    pub borderless_preview_title_background: Color,
    pub preview_title_text: Option<Color>,
    pub borderless_title_text: Color,
    pub borderless_inactive_title_text: Option<Color>,
    pub borderless_title_left: String,
    pub borderless_title_right: String,

    pub text: Color,
    pub directory: Color,
    pub hidden: Color,
    pub unknown_file_icon: Color,
    pub line_number: Color,
    pub current_line_number: Color,

    pub selected: Color,
    pub selected_text: Color,
    pub inactive_selected: Option<Color>,
    pub inactive_selected_text: Option<Color>,
    pub visual_selection_background: Color,
    pub visual_selection_text: Color,

    pub error: Color,
    pub dialog_info: Color,
    pub dialog_warning: Color,
    pub dialog_error: Color,
    pub dialog_info_background: Option<Color>,
    pub dialog_info_text: Option<Color>,
    pub dialog_info_shortcut: Option<Color>,
    pub dialog_warning_background: Option<Color>,
    pub dialog_warning_text: Option<Color>,
    pub dialog_warning_shortcut: Option<Color>,
    pub dialog_error_background: Option<Color>,
    pub dialog_error_text: Option<Color>,
    pub dialog_error_shortcut: Option<Color>,
    pub directory_edit_create: Color,
    pub directory_edit_rename: Color,
    pub directory_edit_delete: Color,

    pub search_match_text: Color,
    pub search_match_background: Color,
    pub search_current_match_text: Color,
    pub search_current_match_background: Color,

    pub preview_background: Color,
    pub preview_separator: Color,
    pub preview_metadata_label: Color,
    pub preview_metadata_value: Color,
    pub preview_unavailable: Color,
    pub preview_visualizer_low: Color,
    pub preview_visualizer_high: Color,

    pub status_background: Color,
    pub compact_status_background: Color,
    pub status_border: Color,
    pub framed_footer_border: Color,
    pub framed_footer_border_follows_mode: bool,
    pub command_background: Color,
    pub mode_normal: Color,
    pub mode_insert: Color,
    pub mode_search: Color,
    pub mode_visual: Color,
    pub mode_command: Color,
    pub compact_mode_text: Color,
    pub compact_mode_background: Option<Color>,
    pub compact_mode_normal_background: Option<Color>,
    terminal_palette: Option<[Rgb; 16]>,
    auto_compact_mode_text: bool,
    pub compact_mode_separator_left: String,
    pub compact_mode_separator_right: String,

    pub border_type: BorderType,
    pub simple_border: bool,
    pub separator_style: SeparatorStyle,
    pub panel_border: BorderStyle,
    pub footer_border: BorderStyle,
    pub decorations: bool,
    pub title_bold: bool,
    pub status_bold: bool,
    pub dim_secondary: bool,

    pub file_color_policy: FileColorPolicy,
    pub file_color_matches: HashMap<Rgb, Color>,
    pub light_file_icons: bool,
    pub icon_variant: IconVariant,
    nearest_file_palette: Option<([Rgb; 16], Rgb, bool)>,
}

#[derive(Clone, Copy)]
pub(crate) struct DialogColors {
    pub background: Color,
    pub text: Color,
    pub accent: Color,
    pub secondary: Color,
}

impl Theme {
    pub fn emergency_monochrome() -> Self {
        let black = Color::from(Rgb {
            red: 0,
            green: 0,
            blue: 0,
        });
        let white = Color::from(Rgb {
            red: 255,
            green: 255,
            blue: 255,
        });

        Self {
            base: black,
            float_background: black,
            active_border: white,
            inactive_border: white,
            active_title: white,
            inactive_title: white,
            framed_active_title_background: None,
            framed_active_title_text: None,
            borderless_active_title_background: white,
            borderless_inactive_title_background: white,
            borderless_preview_title_background: white,
            preview_title_text: None,
            borderless_title_text: black,
            borderless_inactive_title_text: None,
            borderless_title_left: "[".into(),
            borderless_title_right: "]".into(),

            text: white,
            directory: white,
            hidden: white,
            unknown_file_icon: white,
            line_number: white,
            current_line_number: white,

            selected: black,
            selected_text: white,
            inactive_selected: None,
            inactive_selected_text: None,
            visual_selection_background: white,
            visual_selection_text: black,

            error: white,
            dialog_info: white,
            dialog_warning: white,
            dialog_error: white,
            dialog_info_background: None,
            dialog_info_text: None,
            dialog_info_shortcut: None,
            dialog_warning_background: None,
            dialog_warning_text: None,
            dialog_warning_shortcut: None,
            dialog_error_background: None,
            dialog_error_text: None,
            dialog_error_shortcut: None,
            directory_edit_create: white,
            directory_edit_rename: white,
            directory_edit_delete: white,

            search_match_text: black,
            search_match_background: white,
            search_current_match_text: white,
            search_current_match_background: black,

            preview_background: black,
            preview_separator: white,
            preview_metadata_label: white,
            preview_metadata_value: white,
            preview_unavailable: white,
            preview_visualizer_low: white,
            preview_visualizer_high: white,

            status_background: black,
            compact_status_background: black,
            status_border: white,
            framed_footer_border: white,
            framed_footer_border_follows_mode: true,
            command_background: black,
            mode_normal: white,
            mode_insert: white,
            mode_search: white,
            mode_visual: white,
            mode_command: white,
            compact_mode_text: black,
            compact_mode_background: None,
            compact_mode_normal_background: None,
            terminal_palette: None,
            auto_compact_mode_text: false,
            compact_mode_separator_left: ">".into(),
            compact_mode_separator_right: "<".into(),

            border_type: BorderType::Plain,
            simple_border: false,
            separator_style: SeparatorStyle::default(),
            panel_border: BorderStyle::parse("plain").expect("known border style"),
            footer_border: BorderStyle::parse("plain").expect("known border style"),
            decorations: true,
            title_bold: true,
            status_bold: true,
            dim_secondary: false,

            file_color_policy: FileColorPolicy::TerminalDefault,
            file_color_matches: HashMap::new(),
            light_file_icons: false,
            icon_variant: IconVariant::Auto,
            nearest_file_palette: None,
        }
    }

    pub const fn mode_color(&self, mode: StatusMode) -> Color {
        match mode {
            StatusMode::Normal => self.mode_normal,
            StatusMode::Insert => self.mode_insert,
            StatusMode::Search => self.mode_search,
            StatusMode::Visual => self.mode_visual,
            StatusMode::Command => self.mode_command,
        }
    }

    pub(crate) fn compact_mode_background_color(&self, mode: StatusMode) -> Color {
        let normal_background = if mode == StatusMode::Normal {
            self.compact_mode_normal_background
        } else {
            None
        };

        normal_background
            .or(self.compact_mode_background)
            .unwrap_or(self.mode_color(mode))
    }

    pub(crate) fn compact_mode_text_color(&self, mode: StatusMode) -> Color {
        let Some(colors) = self
            .terminal_palette
            .filter(|_| self.auto_compact_mode_text)
        else {
            return self.compact_mode_text;
        };

        let background = match self.compact_mode_background_color(mode) {
            Color::Indexed(index) if index < 16 => colors[usize::from(index)],
            Color::Rgb(red, green, blue) => Rgb { red, green, blue },
            _ => return self.compact_mode_text,
        };

        Color::Indexed(crate::palette_colors::readable_palette_text(background, &colors) as u8)
    }

    pub const fn dialog_color(&self, kind: DialogKind) -> Color {
        match kind {
            DialogKind::Info => self.dialog_info,
            DialogKind::Warning => self.dialog_warning,
            DialogKind::Error => self.dialog_error,
        }
    }

    pub(crate) fn dialog_colors(&self, kind: DialogKind) -> DialogColors {
        let background = match kind {
            DialogKind::Info => self.dialog_info_background,
            DialogKind::Warning => self.dialog_warning_background,
            DialogKind::Error => self.dialog_error_background,
        };
        let text = self.dialog_text_color(kind);

        DialogColors {
            background: background.unwrap_or(self.float_background),
            text: text.unwrap_or(self.text),
            accent: text.unwrap_or(self.dialog_color(kind)),
            secondary: text.unwrap_or(self.inactive_title),
        }
    }

    pub(crate) const fn dialog_text_color(&self, kind: DialogKind) -> Option<Color> {
        match kind {
            DialogKind::Info => self.dialog_info_text,
            DialogKind::Warning => self.dialog_warning_text,
            DialogKind::Error => self.dialog_error_text,
        }
    }

    pub(crate) fn dialog_shortcut_color(&self, kind: DialogKind, fallback: Color) -> Color {
        match kind {
            DialogKind::Info => self.dialog_info_shortcut,
            DialogKind::Warning => self.dialog_warning_shortcut,
            DialogKind::Error => self.dialog_error_shortcut,
        }
        .unwrap_or(fallback)
    }

    pub const fn framed_footer_border_color(&self, mode: StatusMode) -> Color {
        if self.framed_footer_border_follows_mode {
            self.mode_color(mode)
        } else {
            self.framed_footer_border
        }
    }
}

#[cfg(test)]
mod tests;
