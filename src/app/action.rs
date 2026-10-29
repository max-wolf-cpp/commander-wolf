use std::path::PathBuf;

use super::{Entry, Notification};
use crate::directory_edit::DirectoryEditRequest;
use crate::directory_tree::TreeEntry;
use crate::file_opening::Application;
use crate::file_operations::{
    FileOperationBatchRequest, FileOperationOutcome, FileOperationRequest, PanelRefreshResult,
    RefreshContext,
};
use crate::finder::FinderResult;
use crate::icons::IconMode;
use crate::preview::{
    ArchivePreview, DirectoryPreview, DocumentPreview, FileMetadata, ImageViewerLayout,
    MediaMetadata, RasterFrame, TextPreview,
};
use crate::search::SearchDirection;
use crate::status_line::{UiPreset, UiStyle};
use crate::terminal::TerminalSize;
use crate::theme::{BorderStyle, SeparatorStyle};
use crate::workspace::{PanelId, PanelStyle, SelectionMotion};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Initialize,
    Quit,

    DirectoryLoaded {
        panel: PanelId,
        directory: PathBuf,
        entries: Vec<Entry>,
        selected: usize,
    },
    DirectoryLoadFailed {
        panel: PanelId,
        message: String,
    },
    ToggleActivePanel,
    OpenParent,
    OpenSelected,
    OpenSelectedFile,
    RevealSelectedInFileManager,
    CopySelectedPath,
    CopySelectedName,
    CreateArchive(Option<String>),
    ExtractArchive {
        destination: Option<String>,
        other_panel: bool,
    },
    OpenEditorInActiveDirectory(Option<Application>),
    ChangeDirectory(String),
    RefreshPanels,
    ToggleHidden,

    SearchBegin,
    SearchPushCharacter(char),
    SearchBackspace,
    SearchSubmit,
    SearchCancel,
    SearchRepeat {
        direction: SearchDirection,
        count: usize,
    },

    FinderOpen,
    FinderCancel,
    FinderPushCharacter(char),
    FinderBackspace,
    FinderSubmit,
    FinderEnterInsert,
    FinderSelect {
        motion: SelectionMotion,
        count: Option<usize>,
    },
    FinderCenterSelection,
    FinderOpenSelected,
    FinderResultsAdded {
        generation: u64,
        results: Vec<FinderResult>,
    },
    FinderCompleted {
        generation: u64,
        skipped: usize,
        truncated: bool,
        cancelled: bool,
    },
    FinderFailed {
        generation: u64,
        message: String,
    },
    FinderRevealLoaded {
        generation: u64,
        panel: PanelId,
        directory: PathBuf,
        entries: Vec<Entry>,
        selected: usize,
    },
    FinderRevealFailed {
        generation: u64,
        message: String,
    },

    TreeToggle,
    TreeClose,
    TreeSelect {
        motion: SelectionMotion,
        count: Option<usize>,
    },
    TreeCenterSelection,
    TreeToggleSelected,
    TreeExpandSelected,
    TreeCollapseSelected,
    TreeExpandAll,
    TreeCollapseAll,
    TreeOpenSelected,
    TreeChildrenLoaded {
        generation: u64,
        parent: PathBuf,
        entries: Vec<TreeEntry>,
    },
    TreeExpandedAll {
        generation: u64,
        entries: Vec<TreeEntry>,
    },
    TreeLoadFailed {
        generation: u64,
        message: String,
    },
    TreeRevealLoaded {
        generation: u64,
        panel: PanelId,
        directory: PathBuf,
        entries: Vec<Entry>,
        selected: usize,
    },

    EditBegin,
    EditInsertCharacter(char),
    EditNewline,
    EditBackspace,
    EditDelete,
    EditMoveLeft,
    EditMoveRight,
    EditMoveUp,
    EditMoveDown,
    EditHome,
    EditEnd,
    EditRequestExit,
    EditDiscard,
    EditCancelExit,
    DirectoryEditCompleted {
        request: DirectoryEditRequest,
        outcome: FileOperationOutcome,
        results: Box<[PanelRefreshResult; 2]>,
    },

    CopyToOther,
    MoveToOther,
    RequestDelete,

    ToggleMark,
    VisualBegin,
    VisualCancel,

    Rename(String),
    RequestRename {
        force_float: bool,
    },
    RenameInput(crate::dialogs::RenameInput),
    CreateDirectory(String),
    CreateFile(String),

    AcceptConfirmation,
    RejectConfirmation,
    OverwriteFileConflict,
    SkipFileConflict,
    ToggleFileConflictApplyToAll,
    CancelConfirmation,

    FileOperationCompleted {
        request: FileOperationRequest,
        outcome: FileOperationOutcome,
    },
    FileOperationBatchCompleted {
        request: FileOperationBatchRequest,
        outcome: FileOperationOutcome,
    },

    PanelsRefreshed {
        results: [PanelRefreshResult; 2],
        context: RefreshContext,
    },
    WatchedPanelsRefreshed {
        results: Box<[PanelRefreshResult; 2]>,
        changed_paths: Vec<PathBuf>,
    },

    MoveSelection {
        delta: isize,
        count: usize,
    },
    Select {
        motion: SelectionMotion,
        count: Option<usize>,
    },
    CenterSelection,
    TogglePreview,

    SetStylePreset(UiPreset),
    SetFooterStyle(UiStyle),
    SetPanelStyle(PanelStyle),
    SetSeparatorStyle(SeparatorStyle),
    SetBorderStyle(BorderStyle),
    SetIconMode(IconMode),
    SetNameColors(bool),
    SetDynamicFileColorPolicy(crate::theme::DynamicFileColorPolicy),
    UseTheme(String),
    UsePalette(String),
    UseSyntax(String),
    UseDisplayStyle(String),
    ApplyDisplayStyle(crate::display_style::DisplayStyle),

    OpenHelp,
    HelpLoaded(crate::help::HelpState),
    ScrollHelp(isize),
    CloseHelp,

    EnterCommandMode,
    RunShellCommand(String),
    FileOperationProgressStarted(crate::file_operations::progress::OperationProgress),
    FileOperationProgressUpdated(crate::file_operations::progress::OperationProgress),
    CancelFileOperation,
    ShellOutput(String),
    ShellFinished {
        message: String,
        failed: bool,
    },
    CloseShellOutput,
    CancelShellCommand,
    ScrollShellOutput(isize),

    CancelCommandMode,
    PushCommandCharacter(char),
    DeleteCommandCharacter,
    SubmitCommand,
    PreviousCommandHistory,
    NextCommandHistory,
    CompleteCommand {
        backwards: bool,
    },
    CommandCompletionsLoaded {
        line: String,
        items: Vec<crate::commands::completion::CompletionItem>,
        backwards: bool,
    },

    TogglePreviewPlayback,
    ToggleMute,
    PreviewMetadataPageDown,
    PreviewMetadataPageUp,
    PreviewPageNext {
        count: usize,
    },
    PreviewPagePrevious {
        count: usize,
    },
    ViewerOpenSelected,
    ViewerClose,

    Resize {
        size: TerminalSize,
        panel_rows: usize,
        finder_rows: usize,
        tree_rows: usize,
    },

    PreviewMetadataLoaded {
        generation: u64,
        path: PathBuf,
        metadata: FileMetadata,
    },
    PreviewMetadataFailed {
        generation: u64,
        path: PathBuf,
        message: String,
    },
    PreviewMediaPrepared {
        generation: u64,
        path: PathBuf,
        metadata: MediaMetadata,
    },
    PreviewMediaUnavailable {
        generation: u64,
        path: PathBuf,
        message: String,
    },
    PreviewTextLoaded {
        generation: u64,
        path: PathBuf,
        preview: TextPreview,
    },
    PreviewTextUnavailable {
        generation: u64,
        path: PathBuf,
        message: String,
    },
    PreviewDirectoryLoaded {
        generation: u64,
        path: PathBuf,
        preview: DirectoryPreview,
    },
    PreviewDirectoryFailed {
        generation: u64,
        path: PathBuf,
        message: String,
    },
    PreviewArchiveLoaded {
        generation: u64,
        path: PathBuf,
        preview: ArchivePreview,
    },
    PreviewArchiveFailed {
        generation: u64,
        path: PathBuf,
        message: String,
    },

    PreviewDocumentLoaded {
        generation: u64,
        request_id: u64,
        path: PathBuf,
        preview: DocumentPreview,
        raster: Option<RasterFrame>,
    },
    PreviewDocumentFailed {
        generation: u64,
        request_id: u64,
        path: PathBuf,
        message: String,
    },
    PreviewDocumentRasterPresented {
        generation: u64,
        request_id: u64,
        path: PathBuf,
    },

    ViewerProbed {
        generation: u64,
        path: PathBuf,
        metadata: MediaMetadata,
        layout: ImageViewerLayout,
    },
    ViewerPrepared {
        generation: u64,
        path: PathBuf,
    },
    ViewerFailed {
        generation: u64,
        path: PathBuf,
        message: String,
    },

    PreviewPlaybackStarted {
        generation: u64,
        path: PathBuf,
        kind: crate::preview::PreviewKind,
        has_audio: bool,
    },
    PreviewPlaybackStopped,
    PreviewAudioSnapshot {
        generation: u64,
        path: PathBuf,
        position: std::time::Duration,
        muted: bool,
        levels: Vec<u16>,
    },
    PreviewPlaybackFinished {
        generation: u64,
        path: PathBuf,
    },
    PreviewPlaybackFailed {
        generation: u64,
        path: PathBuf,
        message: String,
    },
    ExternalEditorClosed,

    Notify(Notification),
    ClearNotification,
    SetPendingKeys(String),
}
