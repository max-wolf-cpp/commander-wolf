use std::collections::HashMap;

use ratatui::widgets::BorderType;

use super::borders::parse_border_type;
use super::{
    BorderStyle, FileColorPolicy, IconVariant, SeparatorStyle, Theme, format_theme_color,
    parse_theme_color,
};

impl Theme {
    pub fn from_fields(fields: &HashMap<String, String>) -> Result<Self, String> {
        let color = |name: &str| {
            fields
                .get(name)
                .ok_or_else(|| format!("theme is missing '{name}'"))
                .and_then(|value| parse_theme_color(value))
        };
        let flag_or = |name: &str, fallback: bool| {
            fields.get(name).map_or(Ok(fallback), |value| {
                value
                    .parse::<bool>()
                    .map_err(|_| format!("theme field '{name}' must be a boolean"))
            })
        };
        let text_or = |name: &str, fallback: &str| {
            fields
                .get(name)
                .cloned()
                .unwrap_or_else(|| fallback.to_owned())
        };
        let color_or = |name: &str, fallback: &str| {
            fields
                .get(name)
                .or_else(|| fields.get(fallback))
                .ok_or_else(|| format!("theme is missing '{name}' and fallback '{fallback}'"))
                .and_then(|value| parse_theme_color(value))
        };

        let border_type = fields
            .get("border_style")
            .map_or(Ok(BorderType::Plain), |value| parse_border_type(value))?;

        Ok(Self {
            base: color("base")?,
            float_background: color("float_background")?,
            active_border: color("active_border")?,
            inactive_border: color("inactive_border")?,
            active_title: color("active_title")?,
            inactive_title: color("inactive_title")?,
            framed_active_title_background: fields
                .get("framed_active_title_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            framed_active_title_text: fields
                .get("framed_active_title_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            borderless_active_title_background: color_or(
                "borderless_active_title_background",
                "active_title",
            )?,
            borderless_inactive_title_background: color_or(
                "borderless_inactive_title_background",
                "inactive_border",
            )?,
            borderless_preview_title_background: color_or(
                "borderless_preview_title_background",
                "preview_metadata_label",
            )?,
            preview_title_text: fields
                .get("preview_title_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            borderless_title_text: color_or("borderless_title_text", "base")?,
            borderless_inactive_title_text: fields
                .get("borderless_inactive_title_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            borderless_title_left: text_or("borderless_title_left", ""),
            borderless_title_right: text_or("borderless_title_right", ""),

            text: color("text")?,
            directory: color_or("directory", "active_border")?,
            hidden: color_or("hidden", "inactive_title")?,
            unknown_file_icon: color_or("unknown_file_icon", "text")?,
            line_number: color_or("line_number", "inactive_border")?,
            current_line_number: color_or("current_line_number", "active_title")?,

            selected: color("selected")?,
            selected_text: color_or("selected_text", "text")?,
            inactive_selected: fields
                .get("inactive_selected")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            inactive_selected_text: fields
                .get("inactive_selected_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            visual_selection_background: color_or("visual_selection_background", "selected")?,
            visual_selection_text: color_or("visual_selection_text", "text")?,

            error: color("error")?,
            dialog_info: color_or("dialog_info", "active_border")?,
            dialog_warning: color_or("dialog_warning", "mode_command")?,
            dialog_error: color_or("dialog_error", "error")?,
            dialog_info_shortcut: fields
                .get("dialog_info_shortcut")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_info_background: fields
                .get("dialog_info_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_info_text: fields
                .get("dialog_info_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_warning_shortcut: fields
                .get("dialog_warning_shortcut")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_warning_background: fields
                .get("dialog_warning_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_warning_text: fields
                .get("dialog_warning_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_error_shortcut: fields
                .get("dialog_error_shortcut")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_error_background: fields
                .get("dialog_error_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            dialog_error_text: fields
                .get("dialog_error_text")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            directory_edit_create: fields.get("directory_edit_create").map_or_else(
                || color_or("mode_insert", "active_title"),
                |value| parse_theme_color(value),
            )?,
            directory_edit_rename: fields.get("directory_edit_rename").map_or_else(
                || color_or("dialog_warning", "mode_command"),
                |value| parse_theme_color(value),
            )?,
            directory_edit_delete: color_or("directory_edit_delete", "error")?,

            search_match_text: color("search_match_text")?,
            search_match_background: color("search_match_background")?,
            search_current_match_text: color("search_current_match_text")?,
            search_current_match_background: color("search_current_match_background")?,

            preview_background: color("preview_background")?,
            preview_separator: color("preview_separator")?,
            preview_metadata_label: color("preview_metadata_label")?,
            preview_metadata_value: color("preview_metadata_value")?,
            preview_unavailable: color("preview_unavailable")?,
            preview_visualizer_low: color_or("preview_visualizer_low", "inactive_title")?,
            preview_visualizer_high: color_or("preview_visualizer_high", "active_title")?,

            status_background: color_or("status_background", "base")?,
            compact_status_background: color_or("compact_status_background", "status_background")?,
            status_border: color_or("status_border", "inactive_border")?,
            framed_footer_border: color_or("framed_footer_border", "status_border")?,
            framed_footer_border_follows_mode: flag_or("framed_footer_border_follows_mode", true)?,
            command_background: color_or("command_background", "base")?,
            mode_normal: color_or("mode_normal", "active_border")?,
            mode_insert: color_or("mode_insert", "active_title")?,
            mode_search: color_or("mode_search", "error")?,
            mode_visual: color_or("mode_visual", "preview_metadata_label")?,
            mode_command: color_or("mode_command", "active_title")?,
            compact_mode_text: color_or("compact_mode_text", "base")?,
            compact_mode_background: fields
                .get("compact_mode_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            compact_mode_normal_background: fields
                .get("compact_mode_normal_background")
                .map(|value| parse_theme_color(value))
                .transpose()?,
            terminal_palette: None,
            auto_compact_mode_text: false,
            compact_mode_separator_left: text_or("compact_mode_separator_left", ""),
            compact_mode_separator_right: text_or("compact_mode_separator_right", ""),
            border_type,
            simple_border: fields
                .get("border_style")
                .is_some_and(|value| value == "simple"),
            separator_style: fields
                .get("separator_style")
                .map_or(Ok(SeparatorStyle::default()), |value| {
                    SeparatorStyle::parse(value)
                })?,
            panel_border: BorderStyle::parse(
                fields.get("border_style").map_or("plain", String::as_str),
            )?,
            footer_border: BorderStyle::parse(
                fields.get("border_style").map_or("plain", String::as_str),
            )?,
            decorations: true,
            title_bold: flag_or("title_bold", true)?,
            status_bold: flag_or("status_bold", true)?,
            dim_secondary: flag_or("dim_secondary", false)?,

            file_color_policy: fields
                .get("file_color_policy")
                .map_or(Ok(FileColorPolicy::DeviconsRgb), |value| {
                    FileColorPolicy::parse(value)
                })?,
            file_color_matches: HashMap::new(),
            light_file_icons: false,
            icon_variant: fields
                .get("icon_variant")
                .map_or(Ok(IconVariant::Auto), |value| IconVariant::parse(value))?,
            nearest_file_palette: None,
        })
    }

    pub fn apply_palette_fields(&mut self, fields: &HashMap<String, String>) -> Result<(), String> {
        for (key, value) in fields {
            self.apply_palette_field(key, value)?;
        }

        Ok(())
    }

    pub(crate) fn palette_field(&self, key: &str) -> Result<Option<String>, String> {
        let color = match key {
            "base" => Some(self.base),
            "float_background" => Some(self.float_background),
            "active_border" => Some(self.active_border),
            "inactive_border" => Some(self.inactive_border),
            "active_title" => Some(self.active_title),
            "inactive_title" => Some(self.inactive_title),
            "framed_active_title_background" => self.framed_active_title_background,
            "framed_active_title_text" => self.framed_active_title_text,
            "borderless_active_title_background" => Some(self.borderless_active_title_background),
            "borderless_inactive_title_background" => {
                Some(self.borderless_inactive_title_background)
            }
            "borderless_preview_title_background" => Some(self.borderless_preview_title_background),
            "preview_title_text" => self.preview_title_text,
            "borderless_title_text" => Some(self.borderless_title_text),
            "borderless_inactive_title_text" => self.borderless_inactive_title_text,
            "text" => Some(self.text),
            "directory" => Some(self.directory),
            "hidden" => Some(self.hidden),
            "unknown_file_icon" => Some(self.unknown_file_icon),
            "line_number" => Some(self.line_number),
            "current_line_number" => Some(self.current_line_number),
            "selected" => Some(self.selected),
            "selected_text" => Some(self.selected_text),
            "inactive_selected" => self.inactive_selected,
            "inactive_selected_text" => self.inactive_selected_text,
            "visual_selection_background" => Some(self.visual_selection_background),
            "visual_selection_text" => Some(self.visual_selection_text),
            "error" => Some(self.error),
            "dialog_info" => Some(self.dialog_info),
            "dialog_warning" => Some(self.dialog_warning),
            "dialog_error" => Some(self.dialog_error),
            "dialog_info_background" => self.dialog_info_background,
            "dialog_info_text" => self.dialog_info_text,
            "dialog_info_shortcut" => self.dialog_info_shortcut,
            "dialog_warning_background" => self.dialog_warning_background,
            "dialog_warning_text" => self.dialog_warning_text,
            "dialog_warning_shortcut" => self.dialog_warning_shortcut,
            "dialog_error_background" => self.dialog_error_background,
            "dialog_error_text" => self.dialog_error_text,
            "dialog_error_shortcut" => self.dialog_error_shortcut,
            "directory_edit_create" => Some(self.directory_edit_create),
            "directory_edit_rename" => Some(self.directory_edit_rename),
            "directory_edit_delete" => Some(self.directory_edit_delete),
            "search_match_text" => Some(self.search_match_text),
            "search_match_background" => Some(self.search_match_background),
            "search_current_match_text" => Some(self.search_current_match_text),
            "search_current_match_background" => Some(self.search_current_match_background),
            "preview_background" => Some(self.preview_background),
            "preview_separator" => Some(self.preview_separator),
            "preview_metadata_label" => Some(self.preview_metadata_label),
            "preview_metadata_value" => Some(self.preview_metadata_value),
            "preview_unavailable" => Some(self.preview_unavailable),
            "preview_visualizer_low" => Some(self.preview_visualizer_low),
            "preview_visualizer_high" => Some(self.preview_visualizer_high),
            "status_background" => Some(self.status_background),
            "compact_status_background" => Some(self.compact_status_background),
            "status_border" => Some(self.status_border),
            "framed_footer_border" => Some(self.framed_footer_border),
            "command_background" => Some(self.command_background),
            "mode_normal" => Some(self.mode_normal),
            "mode_insert" => Some(self.mode_insert),
            "mode_search" => Some(self.mode_search),
            "mode_visual" => Some(self.mode_visual),
            "mode_command" => Some(self.mode_command),
            "compact_mode_text" => Some(self.compact_mode_text),
            "compact_mode_background" => self.compact_mode_background,
            "compact_mode_normal_background" => self.compact_mode_normal_background,
            "icon_variant" => {
                return Ok(Some(
                    match self.icon_variant {
                        IconVariant::Auto => "auto",
                        IconVariant::Light => "light",
                        IconVariant::Dark => "dark",
                    }
                    .into(),
                ));
            }
            _ => return Err(format!("unknown theme property '{key}'")),
        };

        Ok(color.map(format_theme_color))
    }

    fn apply_palette_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        if key == "file_color_policy" {
            self.file_color_policy = FileColorPolicy::parse(value)?;
            return Ok(());
        }

        if key == "icon_variant" {
            self.icon_variant = IconVariant::parse(value)?;
            return Ok(());
        }

        let color =
            parse_theme_color(value).map_err(|error| format!("UI theme field '{key}': {error}"))?;
        match key {
            "base" => self.base = color,
            "float_background" => self.float_background = color,
            "active_border" => self.active_border = color,
            "inactive_border" => self.inactive_border = color,
            "active_title" => self.active_title = color,
            "inactive_title" => self.inactive_title = color,
            "framed_active_title_background" => self.framed_active_title_background = Some(color),
            "framed_active_title_text" => self.framed_active_title_text = Some(color),
            "borderless_active_title_background" => self.borderless_active_title_background = color,
            "borderless_inactive_title_background" => {
                self.borderless_inactive_title_background = color
            }
            "borderless_preview_title_background" => {
                self.borderless_preview_title_background = color
            }
            "preview_title_text" => self.preview_title_text = Some(color),
            "borderless_title_text" => self.borderless_title_text = color,
            "borderless_inactive_title_text" => self.borderless_inactive_title_text = Some(color),
            "text" => self.text = color,
            "directory" => self.directory = color,
            "hidden" => self.hidden = color,
            "unknown_file_icon" => self.unknown_file_icon = color,
            "line_number" => self.line_number = color,
            "current_line_number" => self.current_line_number = color,
            "selected" => self.selected = color,
            "selected_text" => self.selected_text = color,
            "inactive_selected" => self.inactive_selected = Some(color),
            "inactive_selected_text" => self.inactive_selected_text = Some(color),
            "visual_selection_background" => self.visual_selection_background = color,
            "visual_selection_text" => self.visual_selection_text = color,
            "error" => self.error = color,
            "dialog_info" => self.dialog_info = color,
            "dialog_warning" => self.dialog_warning = color,
            "dialog_error" => self.dialog_error = color,
            "dialog_info_background" => self.dialog_info_background = Some(color),
            "dialog_info_text" => self.dialog_info_text = Some(color),
            "dialog_info_shortcut" => self.dialog_info_shortcut = Some(color),
            "dialog_warning_background" => self.dialog_warning_background = Some(color),
            "dialog_warning_text" => self.dialog_warning_text = Some(color),
            "dialog_warning_shortcut" => self.dialog_warning_shortcut = Some(color),
            "dialog_error_background" => self.dialog_error_background = Some(color),
            "dialog_error_text" => self.dialog_error_text = Some(color),
            "dialog_error_shortcut" => self.dialog_error_shortcut = Some(color),
            "directory_edit_create" => self.directory_edit_create = color,
            "directory_edit_rename" => self.directory_edit_rename = color,
            "directory_edit_delete" => self.directory_edit_delete = color,
            "search_match_text" => self.search_match_text = color,
            "search_match_background" => self.search_match_background = color,
            "search_current_match_text" => self.search_current_match_text = color,
            "search_current_match_background" => self.search_current_match_background = color,
            "preview_background" => self.preview_background = color,
            "preview_separator" => self.preview_separator = color,
            "preview_metadata_label" => self.preview_metadata_label = color,
            "preview_metadata_value" => self.preview_metadata_value = color,
            "preview_unavailable" => self.preview_unavailable = color,
            "preview_visualizer_low" => self.preview_visualizer_low = color,
            "preview_visualizer_high" => self.preview_visualizer_high = color,
            "status_background" => self.status_background = color,
            "compact_status_background" => self.compact_status_background = color,
            "status_border" => self.status_border = color,
            "framed_footer_border" => self.framed_footer_border = color,
            "command_background" => self.command_background = color,
            "mode_normal" => self.mode_normal = color,
            "mode_insert" => self.mode_insert = color,
            "mode_search" => self.mode_search = color,
            "mode_visual" => self.mode_visual = color,
            "mode_command" => self.mode_command = color,
            "compact_mode_text" => {
                self.compact_mode_text = color;
                self.auto_compact_mode_text = false;
            }
            "compact_mode_background" => self.compact_mode_background = Some(color),
            "compact_mode_normal_background" => self.compact_mode_normal_background = Some(color),
            _ => return Err(format!("unknown UI theme field '{key}'")),
        }

        Ok(())
    }
}
