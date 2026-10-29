use ratatui::symbols::border::Set;
use ratatui::widgets::BorderType;

use super::Theme;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SeparatorStyle {
    #[default]
    Thin,
    Rounded,
    Double,
    Thick,
    Simple,
    Dashed,
    Off,
}

impl SeparatorStyle {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "thin" | "plain" => Ok(Self::Thin),
            "rounded" => Ok(Self::Rounded),
            "double" => Ok(Self::Double),
            "thick" => Ok(Self::Thick),
            "simple" => Ok(Self::Simple),
            "dashed" => Ok(Self::Dashed),
            "off" => Ok(Self::Off),
            _ => Err(format!(
                "theme field 'separator_style' has unsupported value '{value}'"
            )),
        }
    }

    pub const fn border_type(self) -> BorderType {
        match self {
            Self::Thin | Self::Dashed | Self::Off => BorderType::Plain,
            Self::Rounded => BorderType::Rounded,
            Self::Double => BorderType::Double,
            Self::Thick => BorderType::Thick,
            Self::Simple => BorderType::Plain,
        }
    }

    pub fn border_set(self) -> Set<'static> {
        match self {
            Self::Simple => simple_border_set(),
            Self::Dashed => Set {
                top_left: "┄",
                top_right: "┄",
                bottom_left: "┄",
                bottom_right: "┄",
                vertical_left: "┆",
                vertical_right: "┆",
                horizontal_top: "┄",
                horizontal_bottom: "┄",
            },
            _ => self.border_type().to_border_set(),
        }
    }

    pub fn horizontal(self) -> &'static str {
        if self == Self::Off {
            ""
        } else {
            self.border_set().horizontal_top
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorderStyle {
    Simple,
    Unicode(BorderType),
}

impl BorderStyle {
    pub fn border_set(self) -> Set<'static> {
        match self {
            Self::Simple => simple_border_set(),
            Self::Unicode(kind) => kind.to_border_set(),
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        if value == "simple" {
            Ok(Self::Simple)
        } else {
            parse_border_type(value).map(Self::Unicode)
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Unicode(BorderType::Plain) => "plain",
            Self::Unicode(BorderType::Rounded) => "rounded",
            Self::Unicode(BorderType::Double) => "double",
            Self::Unicode(BorderType::Thick) => "thick",
            _ => "plain",
        }
    }
}

impl Theme {
    pub fn set_border_style(&mut self, style: BorderStyle) {
        self.panel_border = style;
        self.footer_border = style;

        match style {
            BorderStyle::Simple => {
                self.border_type = BorderType::Plain;
                self.simple_border = true;
            }
            BorderStyle::Unicode(border_type) => {
                self.border_type = border_type;
                self.simple_border = false;
            }
        }
    }

    pub fn border_set(&self) -> Set<'static> {
        if self.simple_border {
            simple_border_set()
        } else {
            self.border_type.to_border_set()
        }
    }

    pub fn panel_border_set(&self) -> Set<'static> {
        self.panel_border.border_set()
    }

    pub fn footer_border_set(&self) -> Set<'static> {
        self.footer_border.border_set()
    }
}

pub(super) fn parse_border_type(value: &str) -> Result<BorderType, String> {
    match value {
        "simple" => Ok(BorderType::Plain),
        "plain" => Ok(BorderType::Plain),
        "rounded" => Ok(BorderType::Rounded),
        "double" => Ok(BorderType::Double),
        "thick" => Ok(BorderType::Thick),
        _ => Err(format!(
            "theme field 'border_style' has unsupported value '{value}'"
        )),
    }
}

fn simple_border_set() -> Set<'static> {
    Set {
        top_left: "+",
        top_right: "+",
        bottom_left: "+",
        bottom_right: "+",
        vertical_left: "|",
        vertical_right: "|",
        horizontal_top: "-",
        horizontal_bottom: "-",
    }
}
