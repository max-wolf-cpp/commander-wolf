use super::*;

#[test]
fn archive_creation_uses_the_union_of_marks_and_visual_selection() {
    let mut state = archive_state();
    state
        .workspace
        .active_mut()
        .marked
        .insert("left/a.png".into());
    state.workspace.active_mut().selected = 1;
    update(&mut state, Action::VisualBegin);
    let effects = update(&mut state, Action::CreateArchive(Some("backup.zip".into())));
    let Some(Effect::PerformFileOperationBatch(request)) = effects.last() else {
        panic!("archive operation expected");
    };
    assert!(
        matches!(request.operations.as_slice(), [FileOperation::CreateArchive { sources, destination }]
        if sources == &vec![PathBuf::from("left/a.png"), PathBuf::from("left/b.png")]
            && destination == &PathBuf::from("left/backup.zip"))
    );
    assert_eq!(
        request.refresh[0].selection.preferred,
        Some("left/backup.zip".into())
    );
    assert_eq!(state.mode, AppMode::Normal);
    assert!(state.visual_selection.panel.is_none());
}

#[test]
fn bare_archive_opens_a_prefilled_command_and_preserves_quoted_names() {
    let mut state = archive_state();
    state.workspace.active_mut().entries[0].name = "new \"wolf\" 目录.txt".into();
    update(&mut state, Action::CreateArchive(None));
    assert_eq!(state.mode, AppMode::Command);
    let command = crate::commands::parse_command_line(&state.command_line).unwrap();
    assert_eq!(command.name, "archive");
    assert_eq!(command.args, ["new \"wolf\" 目录.txt.zip"]);
}

#[test]
fn extraction_targets_a_separate_folder_in_either_panel_or_an_explicit_path() {
    for (destination, other_panel, expected) in [
        (None, false, "left/backup"),
        (None, true, "right/backup"),
        (
            Some("folder with spaces".into()),
            false,
            "left/folder with spaces",
        ),
    ] {
        let mut state = archive_state();
        let entry = &mut state.workspace.active_mut().entries[0];
        entry.name = "backup.zip".into();
        entry.path = "left/backup.zip".into();
        let effects = update(
            &mut state,
            Action::ExtractArchive {
                destination,
                other_panel,
            },
        );
        assert!(
            matches!(effects.last(), Some(Effect::PerformFileOperation(FileOperationRequest {
            operation: FileOperation::ExtractArchive { source, destination }, ..
        })) if source == &PathBuf::from("left/backup.zip") && destination == &PathBuf::from(expected))
        );
    }
}

#[test]
fn archive_commands_reject_parent_entries_unsupported_formats_and_non_zip_output() {
    let mut state = archive_state();
    assert!(update(&mut state, Action::CreateArchive(Some("backup.7z".into()))).is_empty());
    assert!(state.notification.is_some());
    assert!(
        update(
            &mut state,
            Action::ExtractArchive {
                destination: None,
                other_panel: false
            }
        )
        .is_empty()
    );
    state.workspace.active_mut().entries[0].is_parent = true;
    assert!(update(&mut state, Action::CreateArchive(None)).is_empty());
    state.workspace.active_mut().entries.clear();
    assert!(update(&mut state, Action::CreateArchive(None)).is_empty());
}

fn archive_state() -> AppState {
    let mut state = state();
    for (panel, directory) in [(PanelId::Left, "left"), (PanelId::Right, "right")] {
        let panel = state.workspace.panel_mut(panel);
        panel.directory = directory.into();
        for entry in &mut panel.entries {
            entry.path = PathBuf::from(directory).join(&entry.name);
        }
    }
    state
}
