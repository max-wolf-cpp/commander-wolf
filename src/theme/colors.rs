use ratatui::style::Color;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

pub(crate) const FAR_PALETTE: [Rgb; 16] = [
    rgb24(0x000000),
    rgb24(0x800000),
    rgb24(0x008000),
    rgb24(0x808000),
    rgb24(0x000080),
    rgb24(0x800080),
    rgb24(0x008080),
    rgb24(0xC0C0C0),
    rgb24(0x808080),
    rgb24(0xFF0000),
    rgb24(0x00FF00),
    rgb24(0xFFFF00),
    rgb24(0x0000FF),
    rgb24(0xFF00FF),
    rgb24(0x00FFFF),
    rgb24(0xFFFFFF),
];

impl Rgb {
    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.strip_prefix('#').unwrap_or(value);
        if value.len() != 6 || !value.is_ascii() {
            return Err(format!("expected #RRGGBB color, got {value}"));
        }

        let channel = |start| {
            u8::from_str_radix(&value[start..start + 2], 16)
                .map_err(|_| format!("invalid color: {value}"))
        };

        Ok(Self {
            red: channel(0)?,
            green: channel(2)?,
            blue: channel(4)?,
        })
    }
}

impl From<Rgb> for Color {
    fn from(value: Rgb) -> Self {
        Color::Rgb(value.red, value.green, value.blue)
    }
}

pub(crate) fn parse_theme_color(value: &str) -> Result<Color, String> {
    if value == "default" {
        return Ok(Color::Reset);
    }

    let named_index = match value {
        "black" => Some(0),
        "red" => Some(1),
        "green" => Some(2),
        "yellow" => Some(3),
        "blue" => Some(4),
        "purple" | "magenta" => Some(5),
        "cyan" => Some(6),
        "white" => Some(7),
        "brightBlack" => Some(8),
        "brightRed" => Some(9),
        "brightGreen" => Some(10),
        "brightYellow" => Some(11),
        "brightBlue" => Some(12),
        "brightPurple" | "brightMagenta" => Some(13),
        "brightCyan" => Some(14),
        "brightWhite" => Some(15),
        _ => None,
    };
    if let Some(index) = named_index {
        return Ok(Color::Indexed(index));
    }

    if let Some(index) = value.strip_prefix("ansi") {
        return index
            .parse::<u8>()
            .ok()
            .filter(|index| *index < 16)
            .map(Color::Indexed)
            .ok_or_else(|| format!("expected ansi0 through ansi15, got {value}"));
    }

    Rgb::parse(value).map(Into::into)
}

pub(crate) fn format_theme_color(color: Color) -> String {
    match color {
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
        Color::Indexed(index) => format!("ansi{index}"),
        Color::Reset => "default".into(),
        other => format!("{other:?}"),
    }
}

pub(super) const fn rgb24(value: u32) -> Rgb {
    Rgb {
        red: (value >> 16) as u8,
        green: (value >> 8) as u8,
        blue: value as u8,
    }
}
