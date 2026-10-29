use std::path::{Path, PathBuf};

use crate::dialogs::{DialogKind, DialogOptions, FileConflictState};
use crate::directory_edit::DirectoryEditState;
use crate::directory_tree::DirectoryTreeState;
use crate::file_opening::FileAssociations;
use crate::file_operations::FileOperationBatchRequest;
use crate::finder::{FinderOptions, FinderState};
use crate::preview::{ImageViewerState, PlaybackState, PreviewKind, PreviewOptions, PreviewState};
use crate::search::SearchState;
use crate::status_line::StatusLineOptions;
use crate::terminal::TerminalSize;
use crate::workspace::{DirectoryEntry, PanelId, WorkspaceOptions, WorkspaceState};

pub type Entry = DirectoryEntry;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AppMode {
    #[default]
    Normal,
    Insert,
    Command,
    Confirm,
    Search,
    Visual,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisualSelectionState {
    pub panel: Option<PanelId>,
    pub anchor: usize,
}

impl VisualSelectionState {
    pub fn begin(&mut self, panel: PanelId, anchor: usize) {
        self.panel = Some(panel);
        self.anchor = anchor;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn range(self, panel: PanelId, current: usize) -> Option<std::ops::RangeInclusive<usize>> {
        (self.panel == Some(panel)).then(|| self.anchor.min(current)..=self.anchor.max(current))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationLevel {
    Info,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    pub level: NotificationLevel,
    pub message: String,
}

impl Notification {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: NotificationLevel::Info,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: NotificationLevel::Error,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppState {
    pub syntax_theme: crate::syntax::SyntaxTheme,
    pub syntax_enabled: bool,
    pub initial_path: PathBuf,
    pub workspace: WorkspaceState,
    pub separator_style_override: Option<crate::theme::SeparatorStyle>,
    pub border_style_override: Option<crate::theme::BorderStyle>,

    pub mode: AppMode,
    pub command_line: String,
    pub command_history: crate::commands::history::CommandHistory,
    pub command_completion: Option<crate::commands::completion::CommandCompletion>,

    pub help: Option<crate::help::HelpState>,
    pub shell_output: Option<crate::shell::ShellOutput>,
    pub operation_progress: Option<crate::file_operations::progress::OperationProgress>,
    pub notification: Option<Notification>,

    pub preview: PreviewState,
    pub viewer: ImageViewerState,
    pub preview_options: PreviewOptions,

    pub terminal_size: TerminalSize,
    pub protocol_label: String,

    pub pending_keys: String,
    pub visual_selection: VisualSelectionState,
    pub confirmation: Option<FileOperationBatchRequest>,
    pub file_conflict: Option<FileConflictState>,
    pub rename_prompt: Option<crate::dialogs::RenamePrompt>,
    pub dialog_options: DialogOptions,
    pub dialog_kind: DialogKind,

    pub search: SearchState,
    pub finder: FinderState,
    pub finder_options: FinderOptions,
    pub directory_tree: DirectoryTreeState,
    pub directory_edit: DirectoryEditState,

    pub status_line_options: StatusLineOptions,
    pub file_associations: FileAssociations,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(
        initial_path: PathBuf,
        protocol_label: String,
        terminal_size: TerminalSize,
        workspace_options: WorkspaceOptions,
    ) -> Self {
        Self {
            initial_path,
            syntax_theme: crate::syntax::SyntaxTheme::default(),
            syntax_enabled: true,
            workspace: WorkspaceState::new(workspace_options),
            separator_style_override: None,
            border_style_override: None,

            mode: AppMode::Normal,
            command_line: String::new(),
            command_history: crate::commands::history::CommandHistory::default(),
            command_completion: None,

            help: None,
            shell_output: None,
            operation_progress: None,
            notification: None,

            preview: PreviewState::default(),
            viewer: ImageViewerState::default(),
            preview_options: PreviewOptions::default(),

            terminal_size,
            protocol_label,

            pending_keys: String::new(),
            visual_selection: VisualSelectionState::default(),
            confirmation: None,
            file_conflict: None,
            rename_prompt: None,
            dialog_options: DialogOptions::default(),
            dialog_kind: DialogKind::default(),

            search: SearchState::default(),
            finder: FinderState::default(),
            finder_options: FinderOptions::default(),
            directory_tree: DirectoryTreeState::default(),
            directory_edit: DirectoryEditState::default(),

            status_line_options: StatusLineOptions::default(),
            file_associations: FileAssociations::default(),
            should_quit: false,
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.workspace.active().selected_entry()
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_entry().map(|entry| entry.path.as_path())
    }

    pub fn is_preview_playing(&self) -> bool {
        self.preview.playback == PlaybackState::Playing
    }

    pub fn preview_is_playable(&self) -> bool {
        matches!(self.preview.kind, PreviewKind::Video | PreviewKind::Audio)
    }
}
