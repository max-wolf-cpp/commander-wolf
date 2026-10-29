use super::BuiltinCommand;

type BuiltinDefinition = (&'static str, BuiltinCommand, &'static str);

pub(super) fn builtin_commands() -> impl Iterator<Item = BuiltinDefinition> {
    [
        APPLICATION,
        EDITING,
        PANEL_NAVIGATION,
        FILE_OPERATIONS,
        ARCHIVES,
        CONFIRMATION,
        SEARCH,
        FINDER,
        DIRECTORY_TREE,
        DISPLAY,
        COMMAND_LINE,
        MEDIA,
    ]
    .into_iter()
    .flatten()
    .copied()
}

const APPLICATION: &[BuiltinDefinition] = &[
    ("app.quit", BuiltinCommand::AppQuit, "Quit Commander Wolf"),
    (
        "help.open",
        BuiltinCommand::HelpOpen,
        "Show program and keymap help",
    ),
];

const EDITING: &[BuiltinDefinition] = &[
    (
        "edit.begin",
        BuiltinCommand::EditBegin,
        "Edit the active directory as text",
    ),
    (
        "edit.newline",
        BuiltinCommand::EditNewline,
        "Split the current edit line",
    ),
    (
        "edit.backspace",
        BuiltinCommand::EditBackspace,
        "Delete before the edit cursor",
    ),
    (
        "edit.delete",
        BuiltinCommand::EditDelete,
        "Delete at the edit cursor",
    ),
    (
        "edit.cursor.left",
        BuiltinCommand::EditMoveLeft,
        "Move the edit cursor left",
    ),
    (
        "edit.cursor.right",
        BuiltinCommand::EditMoveRight,
        "Move the edit cursor right",
    ),
    (
        "edit.cursor.up",
        BuiltinCommand::EditMoveUp,
        "Move the edit cursor up",
    ),
    (
        "edit.cursor.down",
        BuiltinCommand::EditMoveDown,
        "Move the edit cursor down",
    ),
    (
        "edit.cursor.home",
        BuiltinCommand::EditHome,
        "Move to the start of the edit line",
    ),
    (
        "edit.cursor.end",
        BuiltinCommand::EditEnd,
        "Move to the end of the edit line",
    ),
    (
        "edit.request_exit",
        BuiltinCommand::EditRequestExit,
        "Ask whether to save directory edits",
    ),
    (
        "editor.open_cwd",
        BuiltinCommand::EditorOpenCwd,
        "Open the active panel directory in a text editor",
    ),
];

const PANEL_NAVIGATION: &[BuiltinDefinition] = &[
    (
        "selection.next",
        BuiltinCommand::SelectionNext,
        "Select the next entry",
    ),
    (
        "selection.previous",
        BuiltinCommand::SelectionPrevious,
        "Select the previous entry",
    ),
    (
        "selection.first",
        BuiltinCommand::SelectionFirst,
        "Select the first entry or counted absolute line",
    ),
    (
        "selection.last",
        BuiltinCommand::SelectionLast,
        "Select the last entry or counted absolute line",
    ),
    (
        "selection.absolute",
        BuiltinCommand::SelectionAbsolute,
        "Select an absolute one-based line",
    ),
    (
        "selection.center",
        BuiltinCommand::SelectionCenter,
        "Center the selected entry",
    ),
    (
        "selection.viewport_top",
        BuiltinCommand::SelectionViewportTop,
        "Move to the viewport top boundary",
    ),
    (
        "selection.viewport_middle",
        BuiltinCommand::SelectionViewportMiddle,
        "Move to the viewport middle",
    ),
    (
        "selection.viewport_bottom",
        BuiltinCommand::SelectionViewportBottom,
        "Move to the viewport bottom boundary",
    ),
    (
        "panel.toggle_active",
        BuiltinCommand::PanelToggleActive,
        "Toggle the active panel",
    ),
    (
        "panel.open_parent",
        BuiltinCommand::PanelOpenParent,
        "Open the active panel parent directory",
    ),
    (
        "panel.open_selected",
        BuiltinCommand::PanelOpenSelected,
        "Open the selected directory",
    ),
    (
        "panel.change_directory",
        BuiltinCommand::PanelChangeDirectory,
        "Change the active panel directory",
    ),
    (
        "panel.refresh",
        BuiltinCommand::PanelRefresh,
        "Refresh both panels",
    ),
    (
        "panel.toggle_hidden",
        BuiltinCommand::PanelToggleHidden,
        "Toggle hidden files and directories in both panels",
    ),
];

const FILE_OPERATIONS: &[BuiltinDefinition] = &[
    (
        "file.open_selected",
        BuiltinCommand::FileOpenSelected,
        "Open the selected file or enter the selected directory",
    ),
    (
        "file.reveal_selected",
        BuiltinCommand::FileRevealSelected,
        "Reveal the selected entry in the system file manager",
    ),
    (
        "file.copy_to_other",
        BuiltinCommand::FileCopyToOther,
        "Copy marked entries or the selected entry to the other panel",
    ),
    (
        "file.copy_path",
        BuiltinCommand::FileCopyPath,
        "Copy the selected entry's full path to the system clipboard",
    ),
    (
        "file.copy_name",
        BuiltinCommand::FileCopyName,
        "Copy the selected entry's name, including its extension, to the system clipboard",
    ),
    (
        "file.move_to_other",
        BuiltinCommand::FileMoveToOther,
        "Move marked entries or the selected entry to the other panel",
    ),
    (
        "file.delete",
        BuiltinCommand::FileDelete,
        "Request permanent deletion of marked entries or the selected entry",
    ),
    (
        "file.toggle_mark",
        BuiltinCommand::FileToggleMark,
        "Toggle the mark on the selected entry",
    ),
    (
        "visual.begin",
        BuiltinCommand::VisualBegin,
        "Begin contiguous visual selection",
    ),
    (
        "visual.cancel",
        BuiltinCommand::VisualCancel,
        "Cancel contiguous visual selection",
    ),
    (
        "file.rename",
        BuiltinCommand::FileRename,
        "Rename the selected entry",
    ),
    (
        "file.rename_prompt",
        BuiltinCommand::FileRenamePrompt,
        "Rename the selected entry in a float",
    ),
    (
        "file.create_directory",
        BuiltinCommand::FileCreateDirectory,
        "Create one directory",
    ),
    (
        "file.create_file",
        BuiltinCommand::FileCreateFile,
        "Create one empty file",
    ),
];

const ARCHIVES: &[BuiltinDefinition] = &[
    (
        "archive.create",
        BuiltinCommand::ArchiveCreate,
        "Create a ZIP from marked entries or the current entry",
    ),
    (
        "archive.extract",
        BuiltinCommand::ArchiveExtract,
        "Extract the current archive into a new folder",
    ),
    (
        "archive.extract_to_other",
        BuiltinCommand::ArchiveExtractToOther,
        "Extract the current archive into a new folder in the other panel",
    ),
];

const CONFIRMATION: &[BuiltinDefinition] = &[
    (
        "confirmation.accept",
        BuiltinCommand::ConfirmationAccept,
        "Accept the pending confirmation",
    ),
    (
        "confirmation.reject",
        BuiltinCommand::ConfirmationReject,
        "Reject the pending operation",
    ),
    (
        "confirmation.overwrite",
        BuiltinCommand::ConfirmationOverwrite,
        "Overwrite the current file-operation conflict",
    ),
    (
        "confirmation.skip",
        BuiltinCommand::ConfirmationSkip,
        "Skip the current file-operation conflict",
    ),
    (
        "confirmation.toggle_all",
        BuiltinCommand::ConfirmationToggleAll,
        "Toggle applying a file-conflict choice to all remaining conflicts",
    ),
    (
        "confirmation.cancel",
        BuiltinCommand::ConfirmationCancel,
        "Cancel the pending confirmation",
    ),
];

const SEARCH: &[BuiltinDefinition] = &[
    (
        "search.begin",
        BuiltinCommand::SearchBegin,
        "Begin panel search",
    ),
    (
        "search.submit",
        BuiltinCommand::SearchSubmit,
        "Confirm panel search",
    ),
    (
        "search.cancel",
        BuiltinCommand::SearchCancel,
        "Cancel panel search",
    ),
    (
        "search.backspace",
        BuiltinCommand::SearchBackspace,
        "Delete a search character",
    ),
    (
        "search.next",
        BuiltinCommand::SearchNext,
        "Repeat search forward",
    ),
    (
        "search.previous",
        BuiltinCommand::SearchPrevious,
        "Repeat search backward",
    ),
];

const FINDER: &[BuiltinDefinition] = &[
    (
        "finder.open",
        BuiltinCommand::FinderOpen,
        "Open recursive file finder",
    ),
    (
        "finder.cancel",
        BuiltinCommand::FinderCancel,
        "Cancel or close file finder",
    ),
    (
        "finder.submit",
        BuiltinCommand::FinderSubmit,
        "Start finder search",
    ),
    (
        "finder.backspace",
        BuiltinCommand::FinderBackspace,
        "Delete a finder pattern character",
    ),
    (
        "finder.enter_insert",
        BuiltinCommand::FinderEnterInsert,
        "Edit the finder pattern",
    ),
    (
        "finder.open_selected",
        BuiltinCommand::FinderOpenSelected,
        "Reveal the selected finder result",
    ),
    (
        "finder.selection.next",
        BuiltinCommand::FinderSelectionNext,
        "Select the next finder result",
    ),
    (
        "finder.selection.previous",
        BuiltinCommand::FinderSelectionPrevious,
        "Select the previous finder result",
    ),
    (
        "finder.selection.first",
        BuiltinCommand::FinderSelectionFirst,
        "Select the first finder result",
    ),
    (
        "finder.selection.last",
        BuiltinCommand::FinderSelectionLast,
        "Select the last finder result",
    ),
    (
        "finder.selection.center",
        BuiltinCommand::FinderSelectionCenter,
        "Center the selected finder result",
    ),
    (
        "finder.selection.viewport_top",
        BuiltinCommand::FinderSelectionViewportTop,
        "Select near the finder viewport top",
    ),
    (
        "finder.selection.viewport_middle",
        BuiltinCommand::FinderSelectionViewportMiddle,
        "Select the finder viewport middle",
    ),
    (
        "finder.selection.viewport_bottom",
        BuiltinCommand::FinderSelectionViewportBottom,
        "Select near the finder viewport bottom",
    ),
];

const DIRECTORY_TREE: &[BuiltinDefinition] = &[
    (
        "tree.toggle",
        BuiltinCommand::TreeToggle,
        "Toggle the directory tree",
    ),
    (
        "tree.close",
        BuiltinCommand::TreeClose,
        "Close the directory tree",
    ),
    (
        "tree.selection.next",
        BuiltinCommand::TreeSelectionNext,
        "Select the next tree entry",
    ),
    (
        "tree.selection.previous",
        BuiltinCommand::TreeSelectionPrevious,
        "Select the previous tree entry",
    ),
    (
        "tree.selection.first",
        BuiltinCommand::TreeSelectionFirst,
        "Select the first tree entry",
    ),
    (
        "tree.selection.last",
        BuiltinCommand::TreeSelectionLast,
        "Select the last tree entry",
    ),
    (
        "tree.selection.center",
        BuiltinCommand::TreeSelectionCenter,
        "Center the selected tree entry",
    ),
    (
        "tree.selection.viewport_top",
        BuiltinCommand::TreeSelectionViewportTop,
        "Select near the tree viewport top",
    ),
    (
        "tree.selection.viewport_middle",
        BuiltinCommand::TreeSelectionViewportMiddle,
        "Select the tree viewport middle",
    ),
    (
        "tree.selection.viewport_bottom",
        BuiltinCommand::TreeSelectionViewportBottom,
        "Select near the tree viewport bottom",
    ),
    (
        "tree.node.toggle",
        BuiltinCommand::TreeToggleSelected,
        "Toggle the selected tree directory",
    ),
    (
        "tree.node.expand",
        BuiltinCommand::TreeExpandSelected,
        "Expand the selected tree directory",
    ),
    (
        "tree.node.collapse",
        BuiltinCommand::TreeCollapseSelected,
        "Collapse the selected tree directory",
    ),
    (
        "tree.expand_all",
        BuiltinCommand::TreeExpandAll,
        "Expand the complete directory tree",
    ),
    (
        "tree.collapse_all",
        BuiltinCommand::TreeCollapseAll,
        "Collapse the complete directory tree",
    ),
    (
        "tree.open_selected",
        BuiltinCommand::TreeOpenSelected,
        "Reveal the selected tree entry in the active panel",
    ),
];

const DISPLAY: &[BuiltinDefinition] = &[
    (
        "layout.set",
        BuiltinCommand::LayoutSet,
        "Switch panel and footer layout for this session",
    ),
    (
        "border.set",
        BuiltinCommand::BorderSet,
        "Switch visible frame type for this session",
    ),
    (
        "separator.set",
        BuiltinCommand::SeparatorSet,
        "Switch divider type for this session",
    ),
    (
        "palette.use",
        BuiltinCommand::PaletteUse,
        "Switch UI colors for this session",
    ),
    (
        "syntax.use",
        BuiltinCommand::SyntaxUse,
        "Switch syntax colors or disable highlighting",
    ),
    (
        "display_style.use",
        BuiltinCommand::DisplayStyleUse,
        "Switch a named display style for this session",
    ),
    (
        "icons.set",
        BuiltinCommand::IconsSet,
        "Switch the icon display mode for this session",
    ),
    (
        "name_colors.set",
        BuiltinCommand::NameColorsSet,
        "Enable or disable icon-colored file names for this session",
    ),
    (
        "file_color_policy.set",
        BuiltinCommand::FileColorPolicySet,
        "Compare nearest and original devicons colors in the generated theme",
    ),
    (
        "theme.use",
        BuiltinCommand::ThemeUse,
        "Switch UI colors and syntax for this session",
    ),
];

const COMMAND_LINE: &[BuiltinDefinition] = &[
    (
        "command.enter",
        BuiltinCommand::CommandEnter,
        "Enter command mode",
    ),
    (
        "command.cancel",
        BuiltinCommand::CommandCancel,
        "Cancel command mode",
    ),
    (
        "command.submit",
        BuiltinCommand::CommandSubmit,
        "Submit command line",
    ),
    (
        "command.backspace",
        BuiltinCommand::CommandBackspace,
        "Delete command-line character",
    ),
    (
        "command.history_previous",
        BuiltinCommand::CommandHistoryPrevious,
        "Recall the previous command",
    ),
    (
        "command.history_next",
        BuiltinCommand::CommandHistoryNext,
        "Recall the next command or restore the draft",
    ),
    (
        "shell.execute",
        BuiltinCommand::ShellExecute,
        "Execute an external shell command in the active directory",
    ),
    (
        "command.complete_next",
        BuiltinCommand::CommandCompleteNext,
        "Complete the command or select the next suggestion",
    ),
    (
        "command.complete_previous",
        BuiltinCommand::CommandCompletePrevious,
        "Complete the command or select the previous suggestion",
    ),
];

const MEDIA: &[BuiltinDefinition] = &[
    (
        "preview.toggle",
        BuiltinCommand::PreviewToggle,
        "Toggle preview in place of the inactive panel",
    ),
    (
        "preview.playback.toggle",
        BuiltinCommand::PreviewVideoToggle,
        "Toggle audio or video preview playback",
    ),
    (
        "preview.video.toggle",
        BuiltinCommand::PreviewVideoToggle,
        "Compatibility alias for preview playback toggle",
    ),
    (
        "preview.audio.toggle_mute",
        BuiltinCommand::PreviewAudioToggleMute,
        "Toggle preview audio mute",
    ),
    (
        "preview.metadata.page_down",
        BuiltinCommand::PreviewMetadataPageDown,
        "Scroll preview metadata down by half a page",
    ),
    (
        "preview.metadata.page_up",
        BuiltinCommand::PreviewMetadataPageUp,
        "Scroll preview metadata up by half a page",
    ),
    (
        "preview.page.next",
        BuiltinCommand::PreviewPageNext,
        "Show the next document preview page",
    ),
    (
        "preview.page.previous",
        BuiltinCommand::PreviewPagePrevious,
        "Show the previous document preview page",
    ),
    (
        "viewer.open_selected",
        BuiltinCommand::ViewerOpenSelected,
        "Open the selected image in a modal viewer",
    ),
    (
        "viewer.close",
        BuiltinCommand::ViewerClose,
        "Close the modal image viewer",
    ),
];
