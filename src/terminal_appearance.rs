use crate::theme::Rgb;

mod responses;

pub use responses::{AppearanceReport, ResponseParser, parse_response, parse_scheme_response};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Scheme {
    #[default]
    Unknown,
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalAppearance {
    pub scheme: Scheme,
    pub background: Option<Rgb>,
    pub palette: Option<[Rgb; 16]>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchemePreference {
    #[default]
    Auto,
    Dark,
    Light,
}

impl SchemePreference {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            _ => Err("terminal.appearance must be 'auto', 'dark', or 'light'".into()),
        }
    }
}

impl TerminalAppearance {
    pub fn resolved(self, preference: SchemePreference) -> Self {
        let inferred = self.background.map(|color| {
            if crate::palette_colors::is_light_background(color) {
                Scheme::Light
            } else {
                Scheme::Dark
            }
        });
        let scheme = match preference {
            SchemePreference::Auto => inferred.unwrap_or(self.scheme),
            SchemePreference::Dark => Scheme::Dark,
            SchemePreference::Light => Scheme::Light,
        };
        let background =
            if preference != SchemePreference::Auto && inferred.is_some_and(|it| it != scheme) {
                None
            } else {
                self.background
            };

        Self {
            scheme,
            background,
            palette: self.palette,
        }
    }
}

/// Performs a blocking query for callers with exclusive access to terminal input.
pub fn probe() -> Option<TerminalAppearance> {
    let request = appearance_query(true);

    fox_media_term::query_terminal(&request, std::time::Duration::from_millis(120))
        .ok()
        .map(|response| reports_from_response(response.bytes()))
        .filter(|appearance| {
            appearance.background.is_some()
                || appearance.scheme != Scheme::Unknown
                || appearance.palette.is_some()
        })
}

pub(crate) fn color_query() -> Vec<u8> {
    // Crossterm's Unix reader does not expose private CSI scheme reports.
    // Query OSC colors during the TUI session and infer light/dark from OSC 11.
    appearance_query(false)
}

fn appearance_query(include_scheme_report: bool) -> Vec<u8> {
    let mut request = b"\x1b]11;?\x07".to_vec();
    if include_scheme_report {
        request.extend_from_slice(b"\x1b[?996n");
    }
    for index in 0..16 {
        request.extend_from_slice(format!("\x1b]4;{index};?\x07").as_bytes());
    }

    request
}

pub fn reports_from_response(bytes: &[u8]) -> TerminalAppearance {
    let mut appearance = TerminalAppearance::default();
    let mut palette = [None; 16];
    let mut parser = ResponseParser::default();

    for report in parser.feed(&bytes[..bytes.len().min(2048)]) {
        match report {
            AppearanceReport::Background(color) => appearance.background = Some(color),
            AppearanceReport::PaletteColor(index, color) => {
                palette[usize::from(index)] = Some(color)
            }
            AppearanceReport::Scheme(scheme) => appearance.scheme = scheme,
        }
    }

    if palette.iter().all(Option::is_some) {
        appearance.palette = Some(palette.map(|color| color.expect("all palette colors present")));
    }

    appearance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_osc_colors_and_rejects_malformed_values() {
        assert_eq!(
            parse_response(b"\x1b]11;rgb:ffff/0000/8080\x07"),
            Some(AppearanceReport::Background(Rgb {
                red: 255,
                green: 0,
                blue: 128
            }))
        );
        assert_eq!(
            parse_response(b"\x1b]11;rgb:f/0/8\x1b\\"),
            Some(AppearanceReport::Background(Rgb {
                red: 255,
                green: 0,
                blue: 136
            }))
        );
        assert!(parse_response(b"\x1b]11;rgb:zz/00/00\x07").is_none());
        assert!(parse_response(b"\x1b]11;rgb:00000/00/00\x07").is_none());
        assert_eq!(
            parse_response(b"\x1b]4;14;rgb:0000/ffff/ffff\x1b\\"),
            Some(AppearanceReport::PaletteColor(
                14,
                Rgb {
                    red: 0,
                    green: 255,
                    blue: 255
                }
            ))
        );
        assert_eq!(
            parse_response(b"\x1b]4;3;#808000\x07"),
            Some(AppearanceReport::PaletteColor(
                3,
                Rgb {
                    red: 128,
                    green: 128,
                    blue: 0
                }
            ))
        );
        assert!(parse_response(b"\x1b]4;16;rgb:ffff/0000/0000\x07").is_none());
    }

    #[test]
    fn collects_only_complete_terminal_palettes() {
        let mut reports = b"\x1b]11;rgb:0000/0000/0000\x07".to_vec();
        for index in 0..16 {
            reports
                .extend_from_slice(format!("\x1b]4;{index};rgb:{index:02x}/00/00\x07").as_bytes());
        }
        let complete = reports_from_response(&reports);
        assert_eq!(
            complete.background,
            Some(Rgb {
                red: 0,
                green: 0,
                blue: 0
            })
        );
        assert_eq!(complete.palette.unwrap()[14].red, 14);
        reports.truncate(reports.len() - b"\x1b]4;15;rgb:0f/00/00\x07".len());
        assert!(reports_from_response(&reports).palette.is_none());
    }

    #[test]
    fn background_beats_report_and_explicit_preference_beats_background() {
        let appearance = TerminalAppearance {
            scheme: Scheme::Dark,
            background: Some(Rgb {
                red: 250,
                green: 250,
                blue: 250,
            }),
            palette: None,
        };
        assert_eq!(
            appearance.resolved(SchemePreference::Auto).scheme,
            Scheme::Light
        );
        assert_eq!(appearance.resolved(SchemePreference::Dark).background, None);
    }

    #[test]
    fn extracts_split_control_replies_and_ignores_unrelated_input() {
        let appearance =
            reports_from_response(b"noise\x1b[?997;2n\x1b]11;rgb:0000/0000/0000\x1b\\tail");
        assert_eq!(appearance.scheme, Scheme::Light);
        assert_eq!(
            appearance.background,
            Some(Rgb {
                red: 0,
                green: 0,
                blue: 0
            })
        );
        assert_eq!(
            appearance.resolved(SchemePreference::Auto).scheme,
            Scheme::Dark
        );
        let mut parser = ResponseParser::default();
        assert!(parser.feed(b"\x1b]11;rgb:ff/00/").is_empty());
        assert_eq!(
            parser.feed(b"80\x07"),
            vec![AppearanceReport::Background(Rgb {
                red: 255,
                green: 0,
                blue: 128
            })]
        );
        assert!(parser.feed(&vec![b'1'; 256]).is_empty());
        assert_eq!(
            parser.feed(b"\x1b]11;rgb:bad\x1b[?997;2n"),
            vec![AppearanceReport::Scheme(Scheme::Light)]
        );
    }
}
