use super::*;

#[test]
fn enter_uses_external_opening_for_non_images_and_configured_image_associations() {
    let mut state = state();
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("notes.md"),
        name: "notes.md".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    assert!(matches!(
        update(&mut state, Action::OpenSelectedFile).as_slice(),
        [Effect::OpenExternalFile {
            path,
            application: None,
            text_editor: None
        }] if path == &PathBuf::from("notes.md")
    ));

    let application =
        crate::file_opening::Application::new("image editor".into(), vec!["--reuse-window".into()])
            .unwrap();
    state
        .file_associations
        .set(["png".into()], application.clone())
        .unwrap();
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("wolf.png"),
        name: "wolf.png".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    assert!(matches!(
        update(&mut state, Action::OpenSelectedFile).as_slice(),
        [Effect::OpenExternalFile {
            path,
            application: Some(actual),
            text_editor: None
        }] if path == &PathBuf::from("wolf.png") && actual == &application
    ));
    assert!(!state.viewer.is_open());

    state.file_associations.clear();
    let editor = crate::file_opening::Application::new("nvim".into(), Vec::new()).unwrap();
    state.file_associations.set_text_editor(editor.clone());
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("notes.txt"),
        name: "notes.txt".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    assert!(matches!(
        update(&mut state, Action::OpenSelectedFile).as_slice(),
        [Effect::OpenExternalFile {
            path,
            application: None,
            text_editor: Some(actual)
        }] if path == &PathBuf::from("notes.txt") && actual == &editor
    ));

    let exact = crate::file_opening::Application::new("code".into(), Vec::new()).unwrap();
    state
        .file_associations
        .set(["txt".into()], exact.clone())
        .unwrap();
    assert!(matches!(
        update(&mut state, Action::OpenSelectedFile).as_slice(),
        [Effect::OpenExternalFile {
            application: Some(actual),
            text_editor: Some(fallback),
            ..
        }] if actual == &exact && fallback == &editor
    ));
    state.file_associations.remove(["txt".into()]).unwrap();

    state.preview_options.use_internal_image_viewer = false;
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("wolf.png"),
        name: "wolf.png".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    assert!(matches!(
        update(&mut state, Action::OpenSelectedFile).as_slice(),
        [Effect::OpenExternalFile {
            path,
            application: None,
            text_editor: Some(actual)
        }] if path == &PathBuf::from("wolf.png") && actual == &editor
    ));
    assert!(!state.viewer.is_open());
}

#[test]
fn reveal_selected_targets_files_and_directories_without_navigating() {
    let mut state = state();
    let workspace = state.workspace.clone();
    assert_eq!(
        update(&mut state, Action::RevealSelectedInFileManager),
        vec![Effect::RevealInFileManager {
            path: PathBuf::from("a.png"),
            is_directory: false,
        }]
    );
    assert_eq!(state.workspace, workspace);

    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("folder"),
        name: "folder".into(),
        is_directory: true,
        is_parent: false,
        is_hidden: false,
    };
    assert_eq!(
        update(&mut state, Action::RevealSelectedInFileManager),
        vec![Effect::RevealInFileManager {
            path: PathBuf::from("folder"),
            is_directory: true,
        }]
    );
}

#[test]
fn editor_command_uses_configured_editor_or_an_explicit_override() {
    let mut state = state();
    state.workspace.active_mut().directory = PathBuf::from("workspace");
    assert!(update(&mut state, Action::OpenEditorInActiveDirectory(None)).is_empty());
    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.message.contains("No text editor configured")
    }));

    let configured =
        crate::file_opening::Application::new("nvim".into(), vec!["--clean".into()]).unwrap();
    state.file_associations.set_text_editor(configured.clone());
    assert_eq!(
        update(&mut state, Action::OpenEditorInActiveDirectory(None)),
        vec![Effect::OpenEditorInDirectory {
            directory: PathBuf::from("workspace"),
            editor: configured,
        }]
    );

    let explicit =
        crate::file_opening::Application::new("code".into(), vec!["--new-window".into()]).unwrap();
    assert_eq!(
        update(
            &mut state,
            Action::OpenEditorInActiveDirectory(Some(explicit.clone())),
        ),
        vec![Effect::OpenEditorInDirectory {
            directory: PathBuf::from("workspace"),
            editor: explicit,
        }]
    );
}
