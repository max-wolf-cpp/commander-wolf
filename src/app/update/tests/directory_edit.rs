use super::*;

#[test]
fn directory_edit_exit_has_save_discard_and_cancel_paths() {
    let mut edit_state = state();
    update(&mut edit_state, Action::EditBegin);
    assert_eq!(edit_state.mode, AppMode::Insert);
    assert!(edit_state.directory_edit.active);

    edit_state.directory_edit.lines[0].text = "renamed.png".into();
    update(&mut edit_state, Action::EditRequestExit);
    assert_eq!(edit_state.mode, AppMode::Confirm);
    assert_eq!(edit_state.dialog_kind, DialogKind::Info);

    update(&mut edit_state, Action::CancelConfirmation);
    assert_eq!(edit_state.mode, AppMode::Insert);
    assert_eq!(edit_state.directory_edit.lines[0].text, "renamed.png");

    update(&mut edit_state, Action::EditRequestExit);
    let effects = update(&mut edit_state, Action::AcceptConfirmation);
    assert!(
        matches!(effects.as_slice(), [Effect::ApplyDirectoryEdit(request)] if matches!(request.operations.as_slice(), [FileOperation::Rename { .. }]))
    );

    let mut state = state();
    update(&mut state, Action::EditBegin);
    state.directory_edit.lines[0].text = "discarded.png".into();
    update(&mut state, Action::EditRequestExit);
    update(&mut state, Action::RejectConfirmation);
    assert_eq!(state.mode, AppMode::Normal);
    assert!(!state.directory_edit.active);
}

#[test]
fn unchanged_directory_edit_exits_without_confirmation() {
    let mut state = state();
    update(&mut state, Action::EditBegin);

    update(&mut state, Action::EditRequestExit);

    assert_eq!(state.mode, AppMode::Normal);
    assert!(!state.directory_edit.active);
    assert!(!state.directory_edit.awaiting_confirmation);
    assert!(state.confirmation.is_none());
}

#[test]
fn command_line_edit_confirmation_clears_previous_notifications() {
    let mut state = state();
    state.dialog_options.display = crate::dialogs::DialogDisplay::CommandLine;
    update(&mut state, Action::EditBegin);
    update(&mut state, Action::EditInsertCharacter('x'));
    state.notification = Some(Notification::info("Previous message"));
    update(&mut state, Action::EditRequestExit);
    assert_eq!(state.mode, AppMode::Confirm);
    assert!(state.notification.is_none());
}
