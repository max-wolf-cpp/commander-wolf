use crate::theme::{Rgb, Theme};

mod catalog;
mod colors;

use catalog::catalog;
pub(crate) use catalog::matched_file_colors;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IconMode {
    #[default]
    NerdFont,
    Unicode,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryIconKind {
    Parent,
    Directory,
    DirectoryOpen,
    File,
    New,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedIcon {
    pub glyph: &'static str,
    pub color: Option<Rgb>,
}

pub fn resolve(mode: IconMode, name: &str, kind: EntryIconKind) -> ResolvedIcon {
    resolve_variant(mode, name, kind, false)
}

pub fn resolve_for_theme(
    mode: IconMode,
    name: &str,
    kind: EntryIconKind,
    theme: &Theme,
) -> ResolvedIcon {
    resolve_variant(mode, name, kind, theme.light_file_icons)
}

impl IconMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "nerd_font" => Ok(Self::NerdFont),
            "unicode" => Ok(Self::Unicode),
            "none" | "ascii" => Ok(Self::None),
            _ => Err(format!(
                "workspace.icons must be 'nerd_font', 'unicode', or 'none', got '{value}'"
            )),
        }
    }

    pub fn decoration(self, nerd_glyph: &str) -> &str {
        if self == Self::NerdFont {
            nerd_glyph
        } else {
            ""
        }
    }

    pub fn directory_suffix(self, name: &str, kind: EntryIconKind) -> &'static str {
        if self == Self::None && kind.is_directory() && !name.ends_with(['/', '\\']) {
            "/"
        } else {
            ""
        }
    }

    pub fn label<'a>(self, name: &'a str, kind: EntryIconKind) -> std::borrow::Cow<'a, str> {
        let suffix = self.directory_suffix(name, kind);
        if suffix.is_empty() {
            name.into()
        } else {
            format!("{name}{suffix}").into()
        }
    }
}

impl EntryIconKind {
    const fn is_directory(self) -> bool {
        matches!(self, Self::Parent | Self::Directory | Self::DirectoryOpen)
    }
}

fn resolve_variant(mode: IconMode, name: &str, kind: EntryIconKind, light: bool) -> ResolvedIcon {
    match mode {
        IconMode::NerdFont => resolve_nerd_font(name, kind, light),
        IconMode::Unicode => ResolvedIcon {
            glyph: match kind {
                EntryIconKind::Parent => "↩",
                EntryIconKind::Directory => "🗀",
                EntryIconKind::DirectoryOpen => "🗁",
                EntryIconKind::File => "🗎",
                EntryIconKind::New => "+",
            },
            color: (kind == EntryIconKind::File)
                .then(|| catalog().file_icon(name, light).color)
                .flatten(),
        },
        IconMode::None => ResolvedIcon {
            glyph: "",
            color: (kind == EntryIconKind::File)
                .then(|| catalog().file_icon(name, light).color)
                .flatten(),
        },
    }
}

fn resolve_nerd_font(name: &str, kind: EntryIconKind, light: bool) -> ResolvedIcon {
    match kind {
        EntryIconKind::Parent => ResolvedIcon {
            glyph: "󰁯",
            color: None,
        },
        EntryIconKind::Directory => ResolvedIcon {
            glyph: "",
            color: None,
        },
        EntryIconKind::DirectoryOpen => ResolvedIcon {
            glyph: "",
            color: None,
        },
        EntryIconKind::New => ResolvedIcon {
            glyph: "",
            color: None,
        },
        EntryIconKind::File => {
            let definition = catalog().file_icon(name, light);

            ResolvedIcon {
                glyph: definition.glyph.as_str(),
                color: definition.color,
            }
        }
    }
}

#[cfg(test)]
mod tests;
