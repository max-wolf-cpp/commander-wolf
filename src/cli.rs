use std::ffi::OsString;
use std::path::PathBuf;

use fox_media_term::Protocol;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arguments {
    pub path: PathBuf,
    pub protocol: Protocol,
    pub clean: bool,
    pub explicit_path: bool,
}

impl Arguments {
    pub fn parse_env() -> Result<Self, String> {
        Self::parse(std::env::args_os().skip(1))
    }

    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut protocol = Protocol::Auto;
        let mut path = None;
        let mut clean = false;
        let mut arguments = arguments.into_iter();

        while let Some(argument) = arguments.next() {
            if argument == "--clean" {
                clean = true;
            } else if argument == "--protocol" {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--protocol requires a value".to_owned())?;
                protocol = parse_protocol(value)?;
            } else if path.is_none() {
                path = Some(PathBuf::from(argument));
            } else {
                return Err("usage: cwolf [--clean] [--protocol PROTOCOL] [path]".to_owned());
            }
        }

        Ok(Self {
            explicit_path: path.is_some(),
            path: path.unwrap_or_else(|| PathBuf::from(".")),
            protocol,
            clean,
        })
    }
}

fn parse_protocol(value: OsString) -> Result<Protocol, String> {
    let value = value
        .to_str()
        .ok_or_else(|| "protocol value is not valid UTF-8".to_owned())?;

    Protocol::parse(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_protocol_and_unicode_path() {
        let arguments = Arguments::parse(
            ["--clean", "--protocol", "hb-tc", "Изображения/狼.png"].map(OsString::from),
        )
        .unwrap();
        assert!(arguments.clean);
        assert!(arguments.explicit_path);
        assert_eq!(arguments.protocol, Protocol::HalfBlockTrueColor);
        assert_eq!(arguments.path, PathBuf::from("Изображения/狼.png"));
    }

    #[test]
    fn distinguishes_an_explicit_current_directory_from_no_path() {
        assert!(!Arguments::parse([]).unwrap().explicit_path);
        assert!(
            Arguments::parse([OsString::from(".")])
                .unwrap()
                .explicit_path
        );
    }
}
