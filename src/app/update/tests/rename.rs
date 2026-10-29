use super::*;

#[test]
fn rename_prompt_prefills_selection_respects_display_and_uses_existing_rename_path() {
    use crate::dialogs::{DialogDisplay, RenameInput};

    for (display, force_float, expected) in [
        (DialogDisplay::Float, false, DialogDisplay::Float),
        (
            DialogDisplay::CommandLine,
            false,
            DialogDisplay::CommandLine,
        ),
        (DialogDisplay::CommandLine, true, DialogDisplay::Float),
    ] {
        for directory in [false, true] {
            let mut state = state();
            state.workspace.active_mut().entries[0].is_directory = directory;
            state.dialog_options.display = display;
            state.notification = Some(Notification::info("Previous message"));

            update(&mut state, Action::RequestRename { force_float });

            let prompt = state.rename_prompt.as_ref().unwrap();
            assert_eq!(prompt.text, "a.png");
            assert_eq!(prompt.cursor, 5);
            assert_eq!(prompt.display, expected);
            assert_eq!(
                state.mode,
                if expected == DialogDisplay::Float {
                    AppMode::Normal
                } else {
                    AppMode::Command
                }
            );
            assert!(state.notification.is_none());

            assert!(update(&mut state, Action::ToggleActivePanel).is_empty());
            assert!(update(&mut state, Action::Quit).is_empty());

            update(&mut state, Action::RenameInput(RenameInput::Clear));
            assert!(update(&mut state, Action::RenameInput(RenameInput::Submit)).is_empty());
            assert!(state.rename_prompt.as_ref().unwrap().error.is_some());

            let name = "новое 'имя'.txt";
            for character in name.chars() {
                update(
                    &mut state,
                    Action::RenameInput(RenameInput::Insert(character)),
                );
            }

            let effects = update(&mut state, Action::RenameInput(RenameInput::Submit));
            assert!(effects.iter().any(|effect| matches!(effect, Effect::PerformFileOperation(request)
                if matches!(&request.operation, FileOperation::Rename { source, destination }
                    if source == &PathBuf::from("a.png") && destination.file_name().unwrap() == name))));
            assert!(state.rename_prompt.is_none());
            assert_eq!(state.mode, AppMode::Normal);
        }
    }
}

#[test]
fn rename_prompt_cancel_unchanged_and_parent_do_not_rename() {
    use crate::dialogs::RenameInput;

    for input in [RenameInput::Cancel, RenameInput::Submit] {
        let mut state = state();
        update(&mut state, Action::RequestRename { force_float: true });
        let effects = update(&mut state, Action::RenameInput(input));
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::PerformFileOperation(_)))
        );
        assert!(state.rename_prompt.is_none());
    }

    let mut state = state();
    state.workspace.active_mut().entries[0].is_parent = true;
    update(&mut state, Action::RequestRename { force_float: false });
    assert!(state.rename_prompt.is_none());
    assert!(
        state
            .notification
            .as_ref()
            .unwrap()
            .message
            .contains("parent")
    );

    state.workspace.active_mut().entries.clear();
    update(&mut state, Action::RequestRename { force_float: false });
    assert!(state.rename_prompt.is_none());
}
