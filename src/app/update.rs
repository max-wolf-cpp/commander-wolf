//! Coordinate action handling while feature reducers own their state transitions.
//! Mode gates run before dispatch; overlay policy runs once after dispatch.

mod modes;
mod refresh;
mod selection;
mod workspace;

mod directory_tree;
mod finder;
mod search;

mod archives;
mod confirmations;
mod directory_edit;
mod file_operations;
mod file_requests;
mod rename;

mod command_line;
mod help;
mod overlays;
mod shell;

mod clipboard;
mod display;
mod documents;
mod file_opening;
mod playback;
mod preview;
mod preview_results;
mod viewer;

use crate::app::{Action, AppState, Effect};
use playback::stop_video;
use preview::clear_preview;

pub fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    let overlay_was_open = overlays::is_open(state);
    let preview_generation = state.preview.generation;

    let effects = update_action(state, action);

    let effects = overlays::sync_preview(state, effects, overlay_was_open, preview_generation);
    if overlays::is_open(state) || effects.contains(&Effect::ClearMediaSurface) {
        // Clearing terminal graphics does not discard the loaded media metadata.
        state.preview.surface_suspended = true;
    }

    effects
}

fn update_action(state: &mut AppState, action: Action) -> Vec<Effect> {
    if !modes::action_allowed(state, &action) {
        return Vec::new();
    }

    // Keep dispatch exhaustive so every new action has an explicit owner.
    match action {
        action @ (Action::FileOperationProgressStarted(..)
        | Action::FileOperationProgressUpdated(..)
        | Action::CancelFileOperation
        | Action::FileOperationCompleted { .. }
        | Action::FileOperationBatchCompleted { .. }) => file_operations::update(state, action),
        action @ (Action::Initialize
        | Action::DirectoryLoaded { .. }
        | Action::DirectoryLoadFailed { .. }
        | Action::ToggleActivePanel
        | Action::OpenParent
        | Action::OpenSelected
        | Action::ChangeDirectory(..)
        | Action::ToggleHidden) => workspace::update(state, action),
        Action::Quit => {
            state.should_quit = true;
            let mut effects = stop_video(state);
            effects.extend(clear_preview(state));
            effects.push(Effect::CancelFinderSearch);
            effects
        }
        action @ (Action::OpenSelectedFile
        | Action::RevealSelectedInFileManager
        | Action::OpenEditorInActiveDirectory(..)
        | Action::ExternalEditorClosed) => file_opening::update(state, action),
        action @ (Action::CopySelectedPath | Action::CopySelectedName) => {
            clipboard::update(state, action)
        }
        action @ (Action::CreateArchive(..) | Action::ExtractArchive { .. }) => {
            archives::update(state, action)
        }
        action @ (Action::RefreshPanels
        | Action::PanelsRefreshed { .. }
        | Action::WatchedPanelsRefreshed { .. }) => refresh::update(state, action),
        action @ (Action::SearchBegin
        | Action::SearchPushCharacter(..)
        | Action::SearchBackspace
        | Action::SearchSubmit
        | Action::SearchCancel
        | Action::SearchRepeat { .. }) => search::update(state, action),
        action @ (Action::FinderOpen
        | Action::FinderCancel
        | Action::FinderPushCharacter(..)
        | Action::FinderBackspace
        | Action::FinderSubmit
        | Action::FinderEnterInsert
        | Action::FinderSelect { .. }
        | Action::FinderCenterSelection
        | Action::FinderOpenSelected
        | Action::FinderResultsAdded { .. }
        | Action::FinderCompleted { .. }
        | Action::FinderFailed { .. }
        | Action::FinderRevealLoaded { .. }
        | Action::FinderRevealFailed { .. }) => finder::update(state, action),
        action @ (Action::TreeToggle
        | Action::TreeClose
        | Action::TreeSelect { .. }
        | Action::TreeCenterSelection
        | Action::TreeToggleSelected
        | Action::TreeExpandSelected
        | Action::TreeCollapseSelected
        | Action::TreeExpandAll
        | Action::TreeCollapseAll
        | Action::TreeOpenSelected
        | Action::TreeChildrenLoaded { .. }
        | Action::TreeExpandedAll { .. }
        | Action::TreeLoadFailed { .. }
        | Action::TreeRevealLoaded { .. }) => directory_tree::update(state, action),
        action @ (Action::EditBegin
        | Action::EditInsertCharacter(..)
        | Action::EditNewline
        | Action::EditBackspace
        | Action::EditDelete
        | Action::EditMoveLeft
        | Action::EditMoveRight
        | Action::EditMoveUp
        | Action::EditMoveDown
        | Action::EditHome
        | Action::EditEnd
        | Action::EditRequestExit
        | Action::EditDiscard
        | Action::EditCancelExit
        | Action::DirectoryEditCompleted { .. }) => directory_edit::update(state, action),
        action @ (Action::CopyToOther
        | Action::MoveToOther
        | Action::Rename(..)
        | Action::CreateDirectory(..)
        | Action::CreateFile(..)) => file_requests::update(state, action),
        action @ (Action::RequestDelete
        | Action::AcceptConfirmation
        | Action::RejectConfirmation
        | Action::OverwriteFileConflict
        | Action::SkipFileConflict
        | Action::ToggleFileConflictApplyToAll
        | Action::CancelConfirmation) => confirmations::update(state, action),
        action @ (Action::ToggleMark
        | Action::VisualBegin
        | Action::VisualCancel
        | Action::MoveSelection { .. }
        | Action::Select { .. }
        | Action::CenterSelection) => selection::update(state, action),
        action @ (Action::RequestRename { .. } | Action::RenameInput(..)) => {
            rename::update(state, action)
        }
        action @ Action::TogglePreview => preview::update(state, action),
        action @ (Action::SetStylePreset(..)
        | Action::SetFooterStyle(..)
        | Action::SetPanelStyle(..)
        | Action::SetSeparatorStyle(..)
        | Action::SetBorderStyle(..)
        | Action::SetIconMode(..)
        | Action::SetNameColors(..)
        | Action::SetDynamicFileColorPolicy(..)
        | Action::UseTheme(..)
        | Action::UsePalette(..)
        | Action::UseSyntax(..)
        | Action::UseDisplayStyle(..)
        | Action::ApplyDisplayStyle(..)
        | Action::Resize { .. }) => display::update(state, action),
        action @ (Action::OpenHelp
        | Action::HelpLoaded(..)
        | Action::ScrollHelp(..)
        | Action::CloseHelp) => help::update(state, action),
        action @ (Action::RunShellCommand(..)
        | Action::ShellOutput(..)
        | Action::ShellFinished { .. }
        | Action::CloseShellOutput
        | Action::CancelShellCommand
        | Action::ScrollShellOutput(..)) => shell::update(state, action),
        action @ (Action::EnterCommandMode
        | Action::CancelCommandMode
        | Action::PushCommandCharacter(..)
        | Action::DeleteCommandCharacter
        | Action::SubmitCommand
        | Action::PreviousCommandHistory
        | Action::NextCommandHistory
        | Action::CompleteCommand { .. }
        | Action::CommandCompletionsLoaded { .. }) => command_line::update(state, action),
        action @ (Action::TogglePreviewPlayback
        | Action::ToggleMute
        | Action::PreviewPlaybackStarted { .. }
        | Action::PreviewPlaybackStopped
        | Action::PreviewAudioSnapshot { .. }
        | Action::PreviewPlaybackFinished { .. }
        | Action::PreviewPlaybackFailed { .. }) => playback::update(state, action),
        action @ (Action::PreviewMetadataPageDown
        | Action::PreviewMetadataPageUp
        | Action::PreviewMetadataLoaded { .. }
        | Action::PreviewMetadataFailed { .. }
        | Action::PreviewMediaPrepared { .. }
        | Action::PreviewMediaUnavailable { .. }
        | Action::PreviewTextLoaded { .. }
        | Action::PreviewTextUnavailable { .. }
        | Action::PreviewDirectoryLoaded { .. }
        | Action::PreviewDirectoryFailed { .. }
        | Action::PreviewArchiveLoaded { .. }
        | Action::PreviewArchiveFailed { .. }) => preview_results::update(state, action),
        action @ (Action::PreviewPageNext { .. }
        | Action::PreviewPagePrevious { .. }
        | Action::PreviewDocumentLoaded { .. }
        | Action::PreviewDocumentFailed { .. }
        | Action::PreviewDocumentRasterPresented { .. }) => documents::update(state, action),
        action @ (Action::ViewerOpenSelected
        | Action::ViewerClose
        | Action::ViewerProbed { .. }
        | Action::ViewerPrepared { .. }
        | Action::ViewerFailed { .. }) => viewer::update(state, action),
        Action::Notify(notification) => {
            state.notification = Some(notification);
            Vec::new()
        }
        Action::ClearNotification => {
            state.notification = None;
            Vec::new()
        }
        Action::SetPendingKeys(keys) => {
            state.pending_keys = keys;
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests;
