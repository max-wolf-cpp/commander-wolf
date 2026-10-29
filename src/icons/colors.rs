use ratatui::style::Color;

use super::{EntryIconKind, ResolvedIcon};
use crate::theme::{FileColorPolicy, Theme};

impl ResolvedIcon {
    pub fn file_label_color(
        self,
        name: &str,
        kind: EntryIconKind,
        theme: &Theme,
        enabled: bool,
    ) -> Color {
        if kind.is_directory() {
            return theme.directory;
        }

        if !enabled || kind != EntryIconKind::File {
            return theme.text;
        }

        match theme.file_color_policy {
            FileColorPolicy::DeviconsRgb => self.color_or(theme.text),
            FileColorPolicy::OriginalDeviconsRgb => {
                self.color.map(Into::into).unwrap_or(theme.text)
            }
            FileColorPolicy::TerminalDefault => theme.text,
            FileColorPolicy::NearestPalette => self
                .color
                .and_then(|color| theme.file_color_matches.get(&color).copied())
                .unwrap_or(theme.text),
            FileColorPolicy::SemanticAnsi | FileColorPolicy::SemanticPalette => {
                semantic_file_color(name, theme)
            }
        }
    }

    pub fn themed_display_color(self, kind: EntryIconKind, theme: &Theme) -> Color {
        if kind.is_directory() {
            theme.directory
        } else if kind == EntryIconKind::File
            && theme.file_color_policy == FileColorPolicy::NearestPalette
        {
            self.color
                .and_then(|color| theme.file_color_matches.get(&color).copied())
                .unwrap_or(theme.unknown_file_icon)
        } else if kind == EntryIconKind::File
            && theme.file_color_policy == FileColorPolicy::OriginalDeviconsRgb
        {
            self.color
                .map(Into::into)
                .unwrap_or(theme.unknown_file_icon)
        } else {
            self.display_color(kind, theme.directory, theme.unknown_file_icon, theme.text)
        }
    }

    pub fn color_or(self, fallback: impl Into<Color>) -> Color {
        let fallback = fallback.into();
        if matches!(fallback, Color::Rgb(..)) {
            self.color.map(Into::into).unwrap_or(fallback)
        } else {
            fallback
        }
    }

    pub fn display_color(
        self,
        kind: EntryIconKind,
        directory: impl Into<Color>,
        unknown_file: impl Into<Color>,
        fallback: impl Into<Color>,
    ) -> Color {
        if kind.is_directory() {
            directory.into()
        } else if kind == EntryIconKind::File {
            self.color_or(unknown_file)
        } else {
            self.color_or(fallback)
        }
    }

    pub fn label_color(
        self,
        kind: EntryIconKind,
        directory: impl Into<Color>,
        fallback: impl Into<Color>,
        color_file_names: bool,
    ) -> Color {
        if kind.is_directory() {
            directory.into()
        } else if color_file_names {
            self.color_or(fallback)
        } else {
            fallback.into()
        }
    }

    pub fn spacer(self) -> &'static str {
        if self.glyph.is_empty() { "" } else { " " }
    }
}

fn semantic_file_color(name: &str, theme: &Theme) -> Color {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let index = match extension.as_str() {
        "rs" | "c" | "h" | "cpp" | "hpp" | "py" | "lua" | "js" | "ts" | "go" | "java" | "sh" => 14,
        "zip" | "7z" | "tar" | "gz" | "rar" | "bz2" | "xz" => 11,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "mp3" | "wav" | "flac" | "mp4" | "mkv" => 13,
        "md" | "txt" | "pdf" | "docx" | "xlsx" | "html" => 10,
        "exe" | "bat" | "cmd" | "com" | "app" => 12,
        _ => 7,
    };

    match theme.file_color_policy {
        FileColorPolicy::SemanticPalette => match index {
            14 => theme.mode_command,
            11 => theme.directory,
            13 => theme.mode_visual,
            10 => theme.mode_insert,
            12 => theme.error,
            _ => theme.hidden,
        },
        _ => Color::Indexed(index),
    }
}
