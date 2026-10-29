use std::path::PathBuf;

use crate::directory_edit::DirectoryEditRequest;
use crate::file_opening::Application;
use crate::file_operations::{
    FileOperationBatchRequest, FileOperationRequest, PanelRefreshRequest, RefreshContext,
};
use crate::finder::FinderSearchRequest;
use crate::preview::{DocumentRequest, ImageViewerLayout, MediaMetadata, RasterFrame};
use crate::terminal::TerminalSize;
use crate::workspace::PanelId;
use fox_media_term::Region;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    SetDynamicFileColorPolicy(crate::theme::DynamicFileColorPolicy),
    UseTheme(String),
    UsePalette(String),
    UseSyntax(String),
    UseDisplayStyle(String),
    LoadHelp,

    LoadDirectory {
        panel: PanelId,
        path: PathBuf,
        preferred: Option<PathBuf>,
    },

    OpenExternalFile {
        path: PathBuf,
        application: Option<Application>,
        text_editor: Option<Application>,
    },
    RevealInFileManager {
        path: PathBuf,
        is_directory: bool,
    },
    OpenEditorInDirectory {
        directory: PathBuf,
        editor: Application,
    },

    CopyPathToClipboard(PathBuf),
    CopyNameToClipboard(PathBuf),

    PerformFileOperation(FileOperationRequest),
    PerformFileOperationBatch(FileOperationBatchRequest),
    ApplyDirectoryEdit(DirectoryEditRequest),
    RefreshPanels {
        requests: [PanelRefreshRequest; 2],
        context: RefreshContext,
    },

    StartFinderSearch(FinderSearchRequest),
    CancelFinderSearch,
    RevealFinderResult {
        generation: u64,
        panel: PanelId,
        path: PathBuf,
    },

    LoadTreeChildren {
        show_hidden: bool,
        generation: u64,
        parent: PathBuf,
    },
    ExpandTreeAll {
        show_hidden: bool,
        generation: u64,
        root: PathBuf,
    },
    RevealTreeEntry {
        generation: u64,
        panel: PanelId,
        path: PathBuf,
        is_directory: bool,
    },

    LoadPreviewMetadata {
        generation: u64,
        path: PathBuf,
    },
    PreparePreviewMedia {
        generation: u64,
        path: PathBuf,
        panel: PanelId,
        region: Region,
    },
    LoadPreviewText {
        generation: u64,
        path: PathBuf,
    },
    LoadPreviewDirectory {
        show_hidden: bool,
        generation: u64,
        path: PathBuf,
    },
    LoadPreviewArchive {
        generation: u64,
        path: PathBuf,
    },

    StartDocumentPreview(DocumentRequest),
    CancelDocumentPreview,
    PresentDocumentRaster {
        generation: u64,
        request_id: u64,
        path: PathBuf,
        panel: PanelId,
        region: Region,
        raster: RasterFrame,
    },
    ClearMediaSurface,

    ProbeViewer {
        generation: u64,
        path: PathBuf,
        title: String,
        size: TerminalSize,
        maximum_percent: u16,
    },
    PrepareViewerMedia {
        generation: u64,
        path: PathBuf,
        metadata: MediaMetadata,
        layout: ImageViewerLayout,
    },

    StartPreviewPlayback {
        generation: u64,
        path: PathBuf,
        size: TerminalSize,
        muted: bool,
        panel: PanelId,
        region: Region,
    },
    StopPreviewPlayback,
    SetPreviewMuted(bool),

    ExecuteCommandLine(String),
    RunShellCommand {
        command: String,
        directory: PathBuf,
    },
    CancelShellCommand,
    CancelFileOperation,
    CompleteCommand {
        line: String,
        backwards: bool,
    },
}
