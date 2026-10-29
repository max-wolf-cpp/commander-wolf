use std::collections::HashMap;

use crate::icons::IconMode;
use crate::status_line::UiStyle;
use crate::theme::{BorderStyle, SeparatorStyle, Theme};
use crate::workspace::PanelStyle;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayStyle {
    pub panels: PanelStyle,
    pub footer: UiStyle,
    pub frames: BorderStyle,
    pub panel_border: BorderStyle,
    pub footer_border: BorderStyle,
    pub separator: SeparatorStyle,
    pub decorations: bool,
    pub icons: IconMode,
    pub color_file_names: bool,
    pub title_bold: bool,
    pub status_bold: bool,
    pub dim_secondary: bool,
    pub framed_footer_border_follows_mode: bool,
    pub panel_title_left: String,
    pub panel_title_right: String,
    pub footer_separator_left: String,
    pub footer_separator_right: String,
}

impl Default for DisplayStyle {
    fn default() -> Self {
        let rounded = BorderStyle::parse("rounded").expect("known border style");

        Self {
            panels: PanelStyle::Framed,
            footer: UiStyle::Framed,
            frames: rounded,
            panel_border: rounded,
            footer_border: rounded,
            separator: SeparatorStyle::Thin,
            decorations: true,
            icons: IconMode::NerdFont,
            color_file_names: false,
            title_bold: true,
            status_bold: true,
            dim_secondary: true,
            framed_footer_border_follows_mode: true,
            panel_title_left: "".into(),
            panel_title_right: "".into(),
            footer_separator_left: "".into(),
            footer_separator_right: "".into(),
        }
    }
}

impl DisplayStyle {
    pub fn from_fields(fields: &HashMap<String, String>, base: Self) -> Result<Self, String> {
        let mut result = base;

        for (key, value) in fields {
            match key.as_str() {
                "panels.layout" => result.panels = PanelStyle::parse(value)?,
                "footer.layout" => result.footer = UiStyle::parse(value)?,
                "frames.border" => result.frames = BorderStyle::parse(value)?,
                "panels.border" => result.panel_border = BorderStyle::parse(value)?,
                "footer.border" => result.footer_border = BorderStyle::parse(value)?,
                "separator" => result.separator = SeparatorStyle::parse(value)?,
                "decorations" => result.decorations = parse_decorations(value)?,
                "icons" => result.icons = IconMode::parse(value)?,
                "color_file_names" => result.color_file_names = parse_bool(key, value)?,
                "title_bold" => result.title_bold = parse_bool(key, value)?,
                "status_bold" => result.status_bold = parse_bool(key, value)?,
                "dim_secondary" => result.dim_secondary = parse_bool(key, value)?,
                "framed_footer_border_follows_mode" => {
                    result.framed_footer_border_follows_mode = parse_bool(key, value)?;
                }
                "panels.title_left" => result.panel_title_left = value.clone(),
                "panels.title_right" => result.panel_title_right = value.clone(),
                "footer.separator_left" => result.footer_separator_left = value.clone(),
                "footer.separator_right" => result.footer_separator_right = value.clone(),
                _ => return Err(format!("unknown style field '{key}'")),
            }
        }

        Ok(result)
    }

    pub fn apply_to_theme(&self, theme: &mut Theme) {
        theme.set_border_style(self.frames);
        theme.panel_border = self.panel_border;
        theme.footer_border = self.footer_border;
        theme.separator_style = self.separator;

        theme.decorations = self.decorations;
        theme.title_bold = self.title_bold;
        theme.status_bold = self.status_bold;
        theme.dim_secondary = self.dim_secondary;
        theme.framed_footer_border_follows_mode = self.framed_footer_border_follows_mode;

        theme.borderless_title_left = self.panel_title_left.clone();
        theme.borderless_title_right = self.panel_title_right.clone();
        theme.compact_mode_separator_left = self.footer_separator_left.clone();
        theme.compact_mode_separator_right = self.footer_separator_right.clone();
    }
}

fn parse_decorations(value: &str) -> Result<bool, String> {
    match value {
        "nerd_font" => Ok(true),
        "none" => Ok(false),
        _ => Err(format!(
            "style decorations must be 'nerd_font' or 'none', got '{value}'"
        )),
    }
}

fn parse_bool(key: &str, value: &str) -> Result<bool, String> {
    value
        .parse()
        .map_err(|_| format!("style field '{key}' must be a boolean"))
}
