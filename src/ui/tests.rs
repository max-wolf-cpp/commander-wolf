use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::icons::IconMode;
use crate::preview::{
    ArchivePreview, ArchivePreviewEntry, DirectoryPreview, DirectoryPreviewEntry,
    ImageViewerLayout, LoadState, MediaMetadata, PreviewKind, TextPreview, ViewerStatus,
};
use crate::status_line::{ModePosition, UiStyle};
use crate::terminal::TerminalSize;
use crate::theme::Rgb;
use crate::workspace::{DirectoryEntry, PanelId, WorkspaceOptions};

use super::*;

mod chrome;
mod finder_rendering;
mod footer;
mod panels;
mod preview_rendering;
mod viewer_rendering;

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}
