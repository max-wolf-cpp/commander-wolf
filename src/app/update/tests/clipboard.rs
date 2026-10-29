use super::*;

#[test]
fn clipboard_commands_use_the_current_entry_in_the_active_panel() {
    let mut state = state();
    state.workspace.active_panel = PanelId::Right;
    let panel = state.workspace.active_mut();
    panel.selected = 1;
    panel.entries[1].path = PathBuf::from("right/новый файл.tar.gz");
    panel.marked.insert(panel.entries[0].path.clone());

    assert_eq!(
        update(&mut state, Action::CopySelectedPath),
        vec![Effect::CopyPathToClipboard(
            "right/новый файл.tar.gz".into()
        )]
    );
    assert_eq!(
        update(&mut state, Action::CopySelectedName),
        vec![Effect::CopyNameToClipboard(
            "right/новый файл.tar.gz".into()
        )]
    );
    assert_eq!(state.workspace.active().marked.len(), 1);
}

#[test]
fn directories_and_parent_entries_can_be_copied_without_navigation() {
    let mut state = state();
    let entry = &mut state.workspace.active_mut().entries[0];
    entry.path = PathBuf::from("parent/folder with spaces");
    entry.name = "..".into();
    entry.is_directory = true;
    entry.is_parent = true;

    assert_eq!(
        update(&mut state, Action::CopySelectedName),
        vec![Effect::CopyNameToClipboard(
            "parent/folder with spaces".into()
        )]
    );
    assert_eq!(
        update(&mut state, Action::CopySelectedPath),
        vec![Effect::CopyPathToClipboard(
            "parent/folder with spaces".into()
        )]
    );
    assert_eq!(state.workspace.active().selected, 0);
}

#[test]
fn clipboard_commands_preserve_visual_selection_and_playback() {
    let mut state = state();
    update(&mut state, Action::VisualBegin);
    state.preview.playback = PlaybackState::Playing;

    assert_eq!(
        update(&mut state, Action::CopySelectedPath),
        vec![Effect::CopyPathToClipboard("a.png".into())]
    );
    assert_eq!(state.mode, AppMode::Visual);
    assert_eq!(state.preview.playback, PlaybackState::Playing);
}

#[test]
fn empty_panels_report_an_error_without_writing_the_clipboard() {
    for action in [Action::CopySelectedPath, Action::CopySelectedName] {
        let mut state = state();
        state.workspace.active_mut().entries.clear();

        assert!(update(&mut state, action).is_empty());
        assert_eq!(
            state.notification,
            Some(Notification::error("No entry selected to copy"))
        );
    }
}
