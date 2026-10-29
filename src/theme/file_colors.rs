use ratatui::style::Color;

use super::{Rgb, Theme};
use crate::terminal_appearance::TerminalAppearance;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileColorPolicy {
    DeviconsRgb,
    OriginalDeviconsRgb,
    SemanticAnsi,
    SemanticPalette,
    NearestPalette,
    TerminalDefault,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IconVariant {
    #[default]
    Auto,
    Light,
    Dark,
}

impl IconVariant {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(format!(
                "icon_variant must be 'auto', 'light', or 'dark', got '{value}'"
            )),
        }
    }
}

impl FileColorPolicy {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "devicons_rgb" => Ok(Self::DeviconsRgb),
            "semantic_ansi" => Ok(Self::SemanticAnsi),
            "semantic_palette" => Ok(Self::SemanticPalette),
            "terminal_default" => Ok(Self::TerminalDefault),
            _ => Err(format!("unknown file color policy '{value}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicFileColorPolicy {
    Nearest,
    Original,
}

impl DynamicFileColorPolicy {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "nearest" => Ok(Self::Nearest),
            "original" => Ok(Self::Original),
            _ => Err(":file-color-policy <nearest|original>".into()),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::Original => "original",
        }
    }
}

impl Theme {
    pub(crate) fn use_nearest_file_palette(
        &mut self,
        colors: [Rgb; 16],
        background: Rgb,
        indexed: bool,
    ) {
        self.file_color_policy = FileColorPolicy::NearestPalette;
        self.nearest_file_palette = Some((colors, background, indexed));
        self.file_color_matches =
            crate::icons::matched_file_colors(&colors, background, indexed, self.light_file_icons);
    }

    pub(crate) fn apply_dynamic_file_color_policy(&mut self, policy: DynamicFileColorPolicy) {
        self.file_color_policy = match policy {
            DynamicFileColorPolicy::Original => FileColorPolicy::OriginalDeviconsRgb,
            DynamicFileColorPolicy::Nearest if self.nearest_file_palette.is_some() => {
                FileColorPolicy::NearestPalette
            }
            DynamicFileColorPolicy::Nearest => FileColorPolicy::TerminalDefault,
        };
    }

    pub(crate) fn resolve_icon_variant(&mut self, appearance: TerminalAppearance) {
        let base = match self.base {
            Color::Rgb(red, green, blue) => Some(Rgb { red, green, blue }),
            Color::Indexed(index) => appearance
                .palette
                .and_then(|colors| colors.get(index as usize).copied()),
            _ => None,
        };

        self.light_file_icons = match self.icon_variant {
            IconVariant::Light => true,
            IconVariant::Dark => false,
            IconVariant::Auto => base
                .or(appearance.background)
                .map(crate::palette_colors::is_light_background)
                .unwrap_or(appearance.scheme == crate::terminal_appearance::Scheme::Light),
        };

        if let Some((colors, background, indexed)) = self.nearest_file_palette {
            self.file_color_matches = crate::icons::matched_file_colors(
                &colors,
                base.unwrap_or(background),
                indexed,
                self.light_file_icons,
            );
        }
    }
}
