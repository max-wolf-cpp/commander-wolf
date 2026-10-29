use crate::workspace::PanelStyle;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ModePosition {
    #[default]
    Left,
    Right,
}

impl ModePosition {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(format!(
                "status.mode_position must be 'left' or 'right', got {value}"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StatusLineOptions {
    pub mode_position: ModePosition,
    pub style: UiStyle,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiStyle {
    #[default]
    Framed,
    Borderless,
    Compact,
}

impl UiStyle {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "framed" => Ok(Self::Framed),
            "borderless" => Ok(Self::Borderless),
            "compact" => Ok(Self::Compact),
            _ => Err(format!(
                "footer style must be 'framed', 'borderless', or 'compact', got '{value}'"
            )),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Framed => "framed",
            Self::Borderless => "borderless",
            Self::Compact => "compact",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiPreset {
    #[default]
    Framed,
    Borderless,
    Compact,
}

impl UiPreset {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "framed" => Ok(Self::Framed),
            "borderless" => Ok(Self::Borderless),
            "compact" => Ok(Self::Compact),
            _ => Err(format!(
                "style preset must be 'framed', 'borderless', or 'compact', got '{value}'"
            )),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Framed => "framed",
            Self::Borderless => "borderless",
            Self::Compact => "compact",
        }
    }

    pub const fn footer_style(self) -> UiStyle {
        match self {
            Self::Framed => UiStyle::Framed,
            Self::Borderless => UiStyle::Borderless,
            Self::Compact => UiStyle::Compact,
        }
    }

    pub const fn panel_style(self) -> PanelStyle {
        match self {
            Self::Framed => PanelStyle::Framed,
            Self::Borderless => PanelStyle::Borderless,
            Self::Compact => PanelStyle::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusMode {
    Normal,
    Insert,
    Search,
    Visual,
    Command,
}

impl StatusMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Search => "SEARCH",
            Self::Visual => "VISUAL",
            Self::Command => "COMMAND",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_position_accepts_only_stable_public_names() {
        assert_eq!(ModePosition::parse("left"), Ok(ModePosition::Left));
        assert_eq!(ModePosition::parse("right"), Ok(ModePosition::Right));
        assert!(ModePosition::parse("center").is_err());
    }

    #[test]
    fn footer_styles_use_stable_public_names() {
        assert_eq!(UiStyle::parse("framed"), Ok(UiStyle::Framed));
        assert_eq!(UiStyle::parse("borderless"), Ok(UiStyle::Borderless));
        assert_eq!(UiStyle::parse("compact"), Ok(UiStyle::Compact));
        assert!(UiStyle::parse("unified").is_err());
        assert_eq!(UiStyle::Framed.name(), "framed");
    }

    #[test]
    fn presets_map_to_the_supported_panel_and_footer_pairs() {
        assert_eq!(UiPreset::parse("framed"), Ok(UiPreset::Framed));
        assert_eq!(UiPreset::parse("borderless"), Ok(UiPreset::Borderless));
        assert_eq!(UiPreset::parse("compact"), Ok(UiPreset::Compact));
        assert!(UiPreset::parse("cornered").is_err());
        assert_eq!(UiPreset::Framed.footer_style(), UiStyle::Framed);
        assert_eq!(UiPreset::Framed.panel_style(), PanelStyle::Framed);
        assert_eq!(UiPreset::Borderless.footer_style(), UiStyle::Borderless);
        assert_eq!(UiPreset::Borderless.panel_style(), PanelStyle::Borderless);
        assert_eq!(UiPreset::Compact.footer_style(), UiStyle::Compact);
        assert_eq!(UiPreset::Compact.panel_style(), PanelStyle::Compact);
    }

    #[test]
    fn every_planned_mode_has_a_stable_label() {
        assert_eq!(StatusMode::Normal.label(), "NORMAL");
        assert_eq!(StatusMode::Insert.label(), "INSERT");
        assert_eq!(StatusMode::Search.label(), "SEARCH");
        assert_eq!(StatusMode::Visual.label(), "VISUAL");
        assert_eq!(StatusMode::Command.label(), "COMMAND");
    }
}
