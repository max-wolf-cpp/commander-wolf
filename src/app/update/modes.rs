use crate::app::{Action, AppState};

pub(super) fn action_allowed(state: &AppState, action: &Action) -> bool {
    if state.rename_prompt.is_some()
        && !matches!(action, Action::RenameInput(_) | Action::Resize { .. })
    {
        return false;
    }

    if state.operation_progress.is_some()
        && !matches!(
            action,
            Action::FileOperationProgressUpdated(_)
                | Action::CancelFileOperation
                | Action::FileOperationCompleted { .. }
                | Action::FileOperationBatchCompleted { .. }
                | Action::DirectoryEditCompleted { .. }
                | Action::Resize { .. }
        )
    {
        return false;
    }

    if state.viewer.is_open() && !viewer_action_allowed(action) {
        return false;
    }

    if state.directory_tree.open && !tree_action_allowed(action) {
        return false;
    }

    if state.finder.is_open() && !finder_action_allowed(action) {
        return false;
    }

    if state.search.active && !search_action_allowed(action) {
        return false;
    }

    if state.directory_edit.active && !edit_action_allowed(action) {
        return false;
    }

    true
}

fn finder_action_allowed(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::Resize { .. }
            | Action::FinderCancel
            | Action::FinderPushCharacter(_)
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
            | Action::FinderRevealFailed { .. }
            | Action::PreviewMetadataLoaded { .. }
            | Action::PreviewMetadataFailed { .. }
            | Action::PreviewMediaPrepared { .. }
            | Action::PreviewMediaUnavailable { .. }
            | Action::PreviewTextLoaded { .. }
            | Action::PreviewTextUnavailable { .. }
            | Action::PreviewDirectoryLoaded { .. }
            | Action::PreviewDirectoryFailed { .. }
            | Action::PreviewArchiveLoaded { .. }
            | Action::PreviewArchiveFailed { .. }
            | Action::PreviewDocumentLoaded { .. }
            | Action::PreviewDocumentFailed { .. }
            | Action::PreviewDocumentRasterPresented { .. }
            | Action::PreviewPlaybackStopped
            | Action::Notify(_)
            | Action::ClearNotification
            | Action::SetPendingKeys(_)
    )
}

fn tree_action_allowed(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::Resize { .. }
            | Action::TreeToggle
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
            | Action::TreeRevealLoaded { .. }
            | Action::PreviewPlaybackStopped
            | Action::Notify(_)
            | Action::ClearNotification
            | Action::SetPendingKeys(_)
    )
}

fn edit_action_allowed(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::FileOperationProgressStarted(_)
            | Action::FileOperationProgressUpdated(_)
            | Action::CancelFileOperation
            | Action::Resize { .. }
            | Action::EditInsertCharacter(_)
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
            | Action::AcceptConfirmation
            | Action::RejectConfirmation
            | Action::OverwriteFileConflict
            | Action::SkipFileConflict
            | Action::ToggleFileConflictApplyToAll
            | Action::CancelConfirmation
            | Action::DirectoryEditCompleted { .. }
            | Action::PreviewPlaybackStopped
            | Action::Notify(_)
            | Action::ClearNotification
            | Action::SetPendingKeys(_)
    )
}

fn search_action_allowed(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::Resize { .. }
            | Action::SearchPushCharacter(_)
            | Action::SearchBackspace
            | Action::SearchSubmit
            | Action::SearchCancel
            | Action::PreviewMetadataLoaded { .. }
            | Action::PreviewMetadataFailed { .. }
            | Action::PreviewMediaPrepared { .. }
            | Action::PreviewMediaUnavailable { .. }
            | Action::PreviewTextLoaded { .. }
            | Action::PreviewTextUnavailable { .. }
            | Action::PreviewDirectoryLoaded { .. }
            | Action::PreviewDirectoryFailed { .. }
            | Action::PreviewArchiveLoaded { .. }
            | Action::PreviewArchiveFailed { .. }
            | Action::PreviewDocumentLoaded { .. }
            | Action::PreviewDocumentFailed { .. }
            | Action::PreviewDocumentRasterPresented { .. }
            | Action::PreviewPlaybackStopped
            | Action::Notify(_)
            | Action::ClearNotification
            | Action::SetPendingKeys(_)
    )
}

fn viewer_action_allowed(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::Resize { .. }
            | Action::ViewerClose
            | Action::ViewerProbed { .. }
            | Action::ViewerPrepared { .. }
            | Action::ViewerFailed { .. }
            | Action::Notify(_)
            | Action::ClearNotification
            | Action::SetPendingKeys(_)
    )
}
