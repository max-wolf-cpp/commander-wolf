mod archives;
mod clipboard;
mod command_line;
mod directory_edit;
mod directory_tree;
mod display;
mod documents;
mod file_opening;
mod file_operations;
mod finder;
mod overlays;
mod playback;
mod preview;
mod refresh;
mod rename;
mod search;
mod selection;
mod viewer;
mod workspace;

use std::path::PathBuf;

use super::refresh::refresh_requests;
use super::update;
use crate::app::{Action, AppMode, AppState, Effect, Notification};
use crate::dialogs::DialogKind;
use crate::directory_tree::TreeEntry;
use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationKind, FileOperationOutcome,
    FileOperationRequest, PanelRefreshResult, RefreshContext,
};
use crate::finder::{FinderEntryKind, FinderMode, FinderResult};
use crate::icons::IconMode;
use crate::preview::{DocumentFormat, DocumentPageContent, LoadState, PlaybackState, PreviewKind};
use crate::search::SearchDirection;
use crate::terminal::TerminalSize;
use crate::workspace::{
    DirectoryEntry, DirectorySnapshot, LineNumberMode, PanelId, SelectionMotion, WorkspaceOptions,
};

fn state() -> AppState {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        WorkspaceOptions {
            color_file_names: false,
            show_hidden: true,
            line_numbers: LineNumberMode::Relative,
            scroll_offset: 10,
            icon_mode: IconMode::NerdFont,
            panel_style: crate::workspace::PanelStyle::Framed,
        },
    );

    state.workspace.set_visible_rows(20);
    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).entries = vec![
            DirectoryEntry {
                path: PathBuf::from("a.png"),
                name: "a.png".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            },
            DirectoryEntry {
                path: PathBuf::from("b.png"),
                name: "b.png".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            },
        ];
    }

    state
}
