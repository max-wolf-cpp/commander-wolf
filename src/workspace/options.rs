use crate::icons::IconMode;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LineNumberMode {
    Off,
    Absolute,
    #[default]
    Relative,
}

impl LineNumberMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "off" => Ok(Self::Off),
            "absolute" => Ok(Self::Absolute),
            "relative" => Ok(Self::Relative),
            _ => Err(format!(
                "workspace.line_numbers must be 'off', 'absolute', or 'relative', got '{value}'"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PanelStyle {
    #[default]
    Framed,
    Borderless,
    Compact,
}

impl PanelStyle {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "framed" => Ok(Self::Framed),
            "borderless" => Ok(Self::Borderless),
            "compact" => Ok(Self::Compact),
            _ => Err(format!(
                "panel style must be 'framed', 'borderless', or 'compact', got '{value}'"
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceOptions {
    pub color_file_names: bool,
    pub show_hidden: bool,
    pub line_numbers: LineNumberMode,
    pub scroll_offset: usize,
    pub icon_mode: IconMode,
    pub panel_style: PanelStyle,
}

impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self {
            color_file_names: false,
            line_numbers: LineNumberMode::Off,
            show_hidden: true,
            scroll_offset: 0,
            icon_mode: IconMode::NerdFont,
            panel_style: PanelStyle::Framed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_styles_use_stable_public_names() {
        assert_eq!(PanelStyle::parse("framed"), Ok(PanelStyle::Framed));
        assert_eq!(PanelStyle::parse("borderless"), Ok(PanelStyle::Borderless));
        assert_eq!(PanelStyle::parse("compact"), Ok(PanelStyle::Compact));
        assert!(PanelStyle::parse("cornered").is_err());
        assert_eq!(PanelStyle::Compact.name(), "compact");
    }
}
