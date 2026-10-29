use super::{BuiltinCommand, CommandInvocation, display, navigation};
use crate::app::Action;
use crate::file_opening::Application;
use crate::search::SearchDirection;

pub(super) fn builtin_action(
    command: BuiltinCommand,
    invocation: &CommandInvocation,
) -> Result<Action, String> {
    use BuiltinCommand::*;

    validate_argument_policy(command, invocation)?;

    let count = invocation.count;
    Ok(match command {
        AppQuit => Action::Quit,

        EditBegin => Action::EditBegin,
        EditNewline => Action::EditNewline,
        EditBackspace => Action::EditBackspace,
        EditDelete => Action::EditDelete,
        EditMoveLeft => Action::EditMoveLeft,
        EditMoveRight => Action::EditMoveRight,
        EditMoveUp => Action::EditMoveUp,
        EditMoveDown => Action::EditMoveDown,
        EditHome => Action::EditHome,
        EditEnd => Action::EditEnd,
        EditRequestExit => Action::EditRequestExit,

        SelectionNext
        | SelectionPrevious
        | SelectionFirst
        | SelectionLast
        | SelectionAbsolute
        | SelectionCenter
        | SelectionViewportTop
        | SelectionViewportMiddle
        | SelectionViewportBottom => navigation::selection_action(command, count),

        PanelToggleActive => Action::ToggleActivePanel,
        PanelToggleHidden => Action::ToggleHidden,
        PanelOpenParent => Action::OpenParent,
        PanelOpenSelected => Action::OpenSelected,
        FileOpenSelected => Action::OpenSelectedFile,
        FileRevealSelected => Action::RevealSelectedInFileManager,
        EditorOpenCwd => open_editor(invocation)?,
        PanelChangeDirectory => {
            Action::ChangeDirectory(required_argument(invocation, ":cd <path>")?)
        }
        PanelRefresh => Action::RefreshPanels,

        FileCopyToOther => Action::CopyToOther,
        FileCopyPath => Action::CopySelectedPath,
        FileCopyName => Action::CopySelectedName,
        FileMoveToOther => Action::MoveToOther,
        FileDelete => Action::RequestDelete,
        FileToggleMark => Action::ToggleMark,
        VisualBegin => Action::VisualBegin,
        VisualCancel => Action::VisualCancel,
        FileRename => rename_selected(invocation)?,
        FileRenamePrompt => Action::RequestRename { force_float: true },
        FileCreateDirectory => {
            Action::CreateDirectory(required_argument(invocation, ":mkdir <path>")?)
        }
        FileCreateFile => Action::CreateFile(required_argument(invocation, ":touch <path>")?),
        ArchiveCreate => Action::CreateArchive(
            (!invocation.arguments.is_empty()).then(|| invocation.arguments.join(" ")),
        ),
        ArchiveExtract => Action::ExtractArchive {
            destination: (!invocation.arguments.is_empty()).then(|| invocation.arguments.join(" ")),
            other_panel: false,
        },
        ArchiveExtractToOther => Action::ExtractArchive {
            destination: None,
            other_panel: true,
        },

        ConfirmationAccept => Action::AcceptConfirmation,
        ConfirmationReject => Action::RejectConfirmation,
        ConfirmationOverwrite => Action::OverwriteFileConflict,
        ConfirmationSkip => Action::SkipFileConflict,
        ConfirmationToggleAll => Action::ToggleFileConflictApplyToAll,
        ConfirmationCancel => Action::CancelConfirmation,

        SearchBegin => Action::SearchBegin,
        SearchSubmit => Action::SearchSubmit,
        SearchCancel => Action::SearchCancel,
        SearchBackspace => Action::SearchBackspace,
        SearchNext => Action::SearchRepeat {
            direction: SearchDirection::Forward,
            count: count.unwrap_or(1),
        },
        SearchPrevious => Action::SearchRepeat {
            direction: SearchDirection::Backward,
            count: count.unwrap_or(1),
        },

        FinderOpen => Action::FinderOpen,
        FinderCancel => Action::FinderCancel,
        FinderSubmit => Action::FinderSubmit,
        FinderBackspace => Action::FinderBackspace,
        FinderEnterInsert => Action::FinderEnterInsert,
        FinderOpenSelected => Action::FinderOpenSelected,
        FinderSelectionNext
        | FinderSelectionPrevious
        | FinderSelectionFirst
        | FinderSelectionLast
        | FinderSelectionCenter
        | FinderSelectionViewportTop
        | FinderSelectionViewportMiddle
        | FinderSelectionViewportBottom => navigation::finder_selection_action(command, count),

        TreeToggle => Action::TreeToggle,
        TreeClose => Action::TreeClose,
        TreeSelectionNext
        | TreeSelectionPrevious
        | TreeSelectionFirst
        | TreeSelectionLast
        | TreeSelectionCenter
        | TreeSelectionViewportTop
        | TreeSelectionViewportMiddle
        | TreeSelectionViewportBottom => navigation::tree_selection_action(command, count),
        TreeToggleSelected => Action::TreeToggleSelected,
        TreeExpandSelected => Action::TreeExpandSelected,
        TreeCollapseSelected => Action::TreeCollapseSelected,
        TreeExpandAll => Action::TreeExpandAll,
        TreeCollapseAll => Action::TreeCollapseAll,
        TreeOpenSelected => Action::TreeOpenSelected,

        PreviewToggle => Action::TogglePreview,
        HelpOpen => Action::OpenHelp,

        LayoutSet | BorderSet | SeparatorSet | PaletteUse | SyntaxUse | DisplayStyleUse
        | IconsSet | NameColorsSet | FileColorPolicySet | ThemeUse => {
            display::display_action(command, invocation)?
        }

        ShellExecute => Action::RunShellCommand(required_argument(invocation, ":!<command>")?),

        CommandEnter => Action::EnterCommandMode,
        CommandCancel => Action::CancelCommandMode,
        CommandSubmit => Action::SubmitCommand,
        CommandBackspace => Action::DeleteCommandCharacter,
        CommandHistoryPrevious => Action::PreviousCommandHistory,
        CommandHistoryNext => Action::NextCommandHistory,
        CommandCompleteNext => Action::CompleteCommand { backwards: false },
        CommandCompletePrevious => Action::CompleteCommand { backwards: true },

        PreviewVideoToggle => Action::TogglePreviewPlayback,
        PreviewAudioToggleMute => Action::ToggleMute,
        PreviewMetadataPageDown => Action::PreviewMetadataPageDown,
        PreviewMetadataPageUp => Action::PreviewMetadataPageUp,
        PreviewPageNext => Action::PreviewPageNext {
            count: count.unwrap_or(1),
        },
        PreviewPagePrevious => Action::PreviewPagePrevious {
            count: count.unwrap_or(1),
        },

        ViewerOpenSelected => Action::ViewerOpenSelected,
        ViewerClose => Action::ViewerClose,
    })
}

fn open_editor(invocation: &CommandInvocation) -> Result<Action, String> {
    let editor = invocation
        .arguments
        .split_first()
        .map(|(program, arguments)| Application::new(program.clone(), arguments.to_vec()))
        .transpose()?;

    Ok(Action::OpenEditorInActiveDirectory(editor))
}

fn rename_selected(invocation: &CommandInvocation) -> Result<Action, String> {
    if invocation.arguments.is_empty() {
        return Ok(Action::RequestRename { force_float: false });
    }

    let name = required_argument(invocation, ":rename <new-name>")?;

    Ok(Action::Rename(name))
}

fn validate_argument_policy(
    command: BuiltinCommand,
    invocation: &CommandInvocation,
) -> Result<(), String> {
    if invocation.arguments.is_empty()
        || matches!(
            command,
            BuiltinCommand::PanelChangeDirectory
                | BuiltinCommand::FileRename
                | BuiltinCommand::FileCreateDirectory
                | BuiltinCommand::FileCreateFile
                | BuiltinCommand::ArchiveCreate
                | BuiltinCommand::ArchiveExtract
                | BuiltinCommand::EditorOpenCwd
                | BuiltinCommand::ShellExecute
                | BuiltinCommand::LayoutSet
                | BuiltinCommand::BorderSet
                | BuiltinCommand::SeparatorSet
                | BuiltinCommand::PaletteUse
                | BuiltinCommand::SyntaxUse
                | BuiltinCommand::DisplayStyleUse
                | BuiltinCommand::IconsSet
                | BuiltinCommand::NameColorsSet
                | BuiltinCommand::FileColorPolicySet
                | BuiltinCommand::ThemeUse
        )
    {
        return Ok(());
    }

    let usage = match command {
        BuiltinCommand::PanelRefresh => ":refresh",
        BuiltinCommand::FileCopyToOther => ":copy",
        BuiltinCommand::FileCopyPath => ":copypath",
        BuiltinCommand::FileCopyName => ":copyname",
        BuiltinCommand::FileMoveToOther => ":move",
        BuiltinCommand::FileDelete => ":delete",
        _ => invocation.id.0.as_str(),
    };

    Err(format!(
        "{} usage: {usage}; this command does not accept arguments",
        invocation.id.0
    ))
}

fn required_argument(invocation: &CommandInvocation, usage: &str) -> Result<String, String> {
    if invocation.arguments.is_empty() {
        Err(format!("{} usage: {usage}", invocation.id.0))
    } else {
        Ok(invocation.arguments.join(" "))
    }
}
