use super::*;

#[test]
fn rename_leader_mapping_opens_float_and_prompt_owns_input() {
    use crate::dialogs::{RenameInput, RenamePrompt};
    let config = test_config();
    let mut state = test_state(&config);

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();

    for character in [' ', 'r'] {
        handle_event(
            AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::NONE,
            ))),
            &state,
            &config,
            &mut resolver,
            &mut actions,
        );
    }
    assert!(actions.contains(&Action::RequestRename { force_float: true }));

    state.rename_prompt = Some(RenamePrompt::new(
        "old".into(),
        "old".into(),
        DialogDisplay::Float,
    ));

    for (code, modifiers, expected) in [
        (
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            Some(RenameInput::Insert('q')),
        ),
        (
            KeyCode::Char('猫'),
            KeyModifiers::NONE,
            Some(RenameInput::Insert('猫')),
        ),
        (
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
            Some(RenameInput::SelectAll),
        ),
        (
            KeyCode::Enter,
            KeyModifiers::NONE,
            Some(RenameInput::Submit),
        ),
        (KeyCode::Esc, KeyModifiers::NONE, Some(RenameInput::Cancel)),
        (KeyCode::F(5), KeyModifiers::NONE, None),
        (KeyCode::Tab, KeyModifiers::NONE, None),
    ] {
        actions.clear();
        handle_event(
            AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                code, modifiers,
            ))),
            &state,
            &config,
            &mut resolver,
            &mut actions,
        );
        assert_eq!(
            actions,
            expected
                .map(Action::RenameInput)
                .into_iter()
                .collect::<VecDeque<_>>()
        );
    }
}

#[test]
fn floating_file_conflict_shortcuts_are_fixed_and_case_insensitive() {
    let key =
        |character| crossterm::event::KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    assert_eq!(
        file_conflict_float_action(key('a'), true),
        Some(Action::ToggleFileConflictApplyToAll)
    );
    assert_eq!(file_conflict_float_action(key('a'), false), None);
    assert_eq!(
        file_conflict_float_action(key('O'), true),
        Some(Action::OverwriteFileConflict)
    );
    assert_eq!(
        file_conflict_float_action(key('s'), true),
        Some(Action::SkipFileConflict)
    );
    assert_eq!(
        file_conflict_float_action(key('C'), true),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(
        file_conflict_float_action(
            crossterm::event::KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            true,
        ),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(file_conflict_float_action(key('y'), true), None);
}

#[test]
fn floating_directory_edit_shortcuts_are_fixed_and_case_insensitive() {
    let key =
        |character| crossterm::event::KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    assert_eq!(
        directory_edit_float_action(key('Y')),
        Some(Action::AcceptConfirmation)
    );
    assert_eq!(
        directory_edit_float_action(key('n')),
        Some(Action::RejectConfirmation)
    );
    assert_eq!(
        directory_edit_float_action(key('C')),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(
        directory_edit_float_action(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        )),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(directory_edit_float_action(key('o')), None);
}

#[test]
fn floating_delete_shortcuts_are_fixed_and_case_insensitive() {
    let key =
        |character| crossterm::event::KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    assert_eq!(
        delete_float_action(key('D')),
        Some(Action::AcceptConfirmation)
    );
    assert_eq!(
        delete_float_action(key('c')),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(
        delete_float_action(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        )),
        Some(Action::CancelConfirmation)
    );
    assert_eq!(delete_float_action(key('y')), None);
}

#[test]
fn help_key_opens_float_and_float_owns_navigation_keys() {
    let config = test_config();
    let mut state = test_state(&config);

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();
    let send =
        |code, state: &AppState, resolver: &mut KeyResolver, actions: &mut VecDeque<Action>| {
            handle_event(
                AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                    code,
                    KeyModifiers::NONE,
                ))),
                state,
                &config,
                resolver,
                actions,
            );
        };
    send(KeyCode::Char('?'), &state, &mut resolver, &mut actions);
    assert!(actions.contains(&Action::OpenHelp));

    state.help = Some(crate::help::HelpState::from_keymaps(&config.keymaps()));
    actions.clear();
    send(KeyCode::Char('j'), &state, &mut resolver, &mut actions);
    assert_eq!(actions.pop_front(), Some(Action::ScrollHelp(1)));
    send(KeyCode::Esc, &state, &mut resolver, &mut actions);
    assert_eq!(actions.pop_front(), Some(Action::CloseHelp));
}

#[test]
fn shell_output_owns_input_until_dismissed() {
    let config = test_config();
    let mut state = test_state(&config);
    state.shell_output = Some(crate::shell::ShellOutput::new("echo hello".into()));

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();
    for key in [
        KeyCode::Char('j'),
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Char(':'),
    ] {
        handle_event(
            AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                key,
                KeyModifiers::NONE,
            ))),
            &state,
            &config,
            &mut resolver,
            &mut actions,
        );
        assert!(actions.is_empty());
    }

    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert_eq!(actions.pop_front(), Some(Action::CancelShellCommand));
    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::NONE,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert_eq!(actions.pop_front(), Some(Action::ScrollShellOutput(1)));

    state.shell_output.as_mut().unwrap().finished = Some("Exit code: 0".into());
    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char(':'),
            KeyModifiers::NONE,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert_eq!(
        actions.into_iter().collect::<Vec<_>>(),
        [Action::CloseShellOutput, Action::EnterCommandMode]
    );
}
