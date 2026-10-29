use super::Scheme;
use crate::theme::Rgb;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppearanceReport {
    Background(Rgb),
    PaletteColor(u8, Rgb),
    Scheme(Scheme),
}

pub fn parse_response(bytes: &[u8]) -> Option<AppearanceReport> {
    let response = bytes.strip_prefix(b"\x1b]")?;
    let response = response
        .strip_suffix(b"\x07")
        .or_else(|| response.strip_suffix(b"\x1b\\"))?;
    if let Some(color) = response.strip_prefix(b"11;") {
        return Some(AppearanceReport::Background(parse_report_color(color)?));
    }

    let body = response.strip_prefix(b"4;")?;
    let separator = body.iter().position(|byte| *byte == b';')?;
    let (index, color) = body.split_at(separator);
    let index = std::str::from_utf8(index).ok()?.parse::<u8>().ok()?;
    if index >= 16 {
        return None;
    }

    Some(AppearanceReport::PaletteColor(
        index,
        parse_report_color(&color[1..])?,
    ))
}

pub fn parse_scheme_response(bytes: &[u8]) -> Option<AppearanceReport> {
    match bytes {
        b"\x1b[?997;1n" => Some(AppearanceReport::Scheme(Scheme::Dark)),
        b"\x1b[?997;2n" => Some(AppearanceReport::Scheme(Scheme::Light)),
        _ => None,
    }
}

#[derive(Default)]
pub struct ResponseParser {
    pending: Vec<u8>,
}

impl ResponseParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<AppearanceReport> {
        bytes
            .iter()
            .filter_map(|&byte| self.feed_byte(byte))
            .collect()
    }

    fn feed_byte(&mut self, byte: u8) -> Option<AppearanceReport> {
        if self.pending.is_empty() {
            if byte == b'\x1b' {
                self.pending.push(byte);
            }
            return None;
        }

        if self.pending.len() == 1 && byte != b']' && byte != b'[' {
            self.pending.clear();
            if byte == b'\x1b' {
                self.pending.push(byte);
            }
            return None;
        }

        if self.pending.len() > 2 && self.pending.last() == Some(&b'\x1b') && byte != b'\\' {
            self.pending.clear();
            self.pending.push(b'\x1b');
            if byte == b']' || byte == b'[' {
                self.pending.push(byte);
            }
            return None;
        }

        if self.pending.len() >= 128 {
            self.pending.clear();
            return None;
        }

        let is_osc = self.pending.get(1) == Some(&b']');
        self.pending.push(byte);
        let complete = if is_osc {
            byte == b'\x07' || self.pending.ends_with(b"\x1b\\")
        } else {
            byte.is_ascii_alphabetic() || byte == b'~'
        };
        if !complete {
            return None;
        }

        let report = if is_osc {
            parse_response(&self.pending)
        } else {
            parse_scheme_response(&self.pending)
        };
        self.pending.clear();

        report
    }
}

fn parse_report_color(value: &[u8]) -> Option<Rgb> {
    if value.starts_with(b"#") {
        return Rgb::parse(std::str::from_utf8(value).ok()?).ok();
    }

    let body = value.strip_prefix(b"rgb:")?;
    let mut channels = body.split(|byte| *byte == b'/');
    let color = Rgb {
        red: parse_channel(channels.next()?)?,
        green: parse_channel(channels.next()?)?,
        blue: parse_channel(channels.next()?)?,
    };

    if let Some(alpha) = channels.next() {
        parse_channel(alpha)?;
    }

    channels.next().is_none().then_some(color)
}

fn parse_channel(component: &[u8]) -> Option<u8> {
    if !(1..=4).contains(&component.len()) || !component.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }

    let value = u16::from_str_radix(std::str::from_utf8(component).ok()?, 16).ok()?;
    let maximum = (1u32 << (component.len() * 4)) - 1;

    Some(((u32::from(value) * 255 + maximum / 2) / maximum) as u8)
}
