use crate::app::Action;
use crate::commands::{CommandDispatch, CommandId, CommandInvocation, CommandRegistry};
use crate::file_opening::Application;

#[test]
fn directory_and_rename_commands_preserve_logical_arguments() {
    assert_eq!(
        dispatch("panel.change_directory", vec!["目录", "with spaces"]).unwrap(),
        CommandDispatch::Actions(vec![Action::ChangeDirectory("目录 with spaces".into())])
    );
    assert_eq!(
        dispatch("file.rename", vec!["новое имя.txt"]).unwrap(),
        CommandDispatch::Actions(vec![Action::Rename("новое имя.txt".into())])
    );
}

#[test]
fn rename_without_a_name_uses_the_existing_prompt() {
    assert_eq!(
        dispatch("file.rename", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::RequestRename { force_float: false }])
    );
    assert_eq!(
        dispatch("file.rename_prompt", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::RequestRename { force_float: true }])
    );
    assert!(dispatch("file.rename_prompt", vec!["extra"]).is_err());
}

#[test]
fn reveal_uses_the_file_manager() {
    assert_eq!(
        dispatch("file.reveal_selected", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::RevealSelectedInFileManager])
    );
}

#[test]
fn file_opening_preserves_the_editor_override() {
    assert_eq!(
        dispatch("editor.open_cwd", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::OpenEditorInActiveDirectory(None)])
    );
    assert_eq!(
        dispatch("editor.open_cwd", vec!["code", "--new-window"]).unwrap(),
        CommandDispatch::Actions(vec![Action::OpenEditorInActiveDirectory(Some(
            Application::new("code".into(), vec!["--new-window".into()]).unwrap()
        ))])
    );
}

#[test]
fn file_commands_require_paths() {
    for (id, usage) in [
        ("panel.change_directory", ":cd <path>"),
        ("file.create_directory", ":mkdir <path>"),
        ("file.create_file", ":touch <path>"),
    ] {
        assert!(dispatch(id, vec![]).unwrap_err().contains(usage));
    }
}

#[test]
fn archive_commands_preserve_destinations_and_allow_the_default_prompt() {
    assert_eq!(
        dispatch("archive.create", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::CreateArchive(None)])
    );
    assert_eq!(
        dispatch("archive.create", vec!["目录 with spaces.zip"]).unwrap(),
        CommandDispatch::Actions(vec![Action::CreateArchive(Some(
            "目录 with spaces.zip".into()
        ))])
    );
    assert_eq!(
        dispatch("archive.extract", vec!["folder with spaces"]).unwrap(),
        CommandDispatch::Actions(vec![Action::ExtractArchive {
            destination: Some("folder with spaces".into()),
            other_panel: false
        }])
    );
    assert_eq!(
        dispatch("archive.extract_to_other", vec![]).unwrap(),
        CommandDispatch::Actions(vec![Action::ExtractArchive {
            destination: None,
            other_panel: true
        }])
    );
    assert!(dispatch("archive.extract_to_other", vec!["extra"]).is_err());
}

#[test]
fn clipboard_commands_dispatch_without_arguments() {
    for (id, action) in [
        ("file.copy_path", Action::CopySelectedPath),
        ("file.copy_name", Action::CopySelectedName),
    ] {
        assert_eq!(
            dispatch(id, vec![]).unwrap(),
            CommandDispatch::Actions(vec![action])
        );
        assert!(dispatch(id, vec!["extra"]).is_err());
    }
}

#[test]
fn file_operations_reject_extra_arguments() {
    for id in [
        "panel.refresh",
        "file.reveal_selected",
        "file.copy_to_other",
        "file.move_to_other",
        "file.delete",
    ] {
        let error = dispatch(id, vec!["extra"]).unwrap_err();

        assert!(error.contains(id));
        assert!(error.contains("does not accept arguments"));
    }
}

fn dispatch(id: &str, arguments: Vec<&str>) -> Result<CommandDispatch, String> {
    CommandRegistry::with_builtins().dispatch(CommandInvocation::new(
        CommandId::from(id),
        arguments.into_iter().map(str::to_owned).collect(),
        None,
    ))
}
