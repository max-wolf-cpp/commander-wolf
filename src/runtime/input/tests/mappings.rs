use super::*;

#[test]
fn command_mode_printable_input_becomes_an_action() {
    let config = test_config();
    let state = test_state(&config);
    let mut command_state = state;
    command_state.mode = AppMode::Command;

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();
    let key = crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    handle_event(
        AppEvent::Input(InputEvent::Key(key)),
        &command_state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert!(actions.contains(&Action::PushCommandCharacter('x')));

    for (key, expected) in [
        (KeyCode::Up, Action::PreviousCommandHistory),
        (KeyCode::Down, Action::NextCommandHistory),
        (KeyCode::Tab, Action::CompleteCommand { backwards: false }),
        (
            KeyCode::BackTab,
            Action::CompleteCommand { backwards: true },
        ),
    ] {
        actions.clear();
        handle_event(
            AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                key,
                KeyModifiers::NONE,
            ))),
            &command_state,
            &config,
            &mut resolver,
            &mut actions,
        );
        assert!(actions.contains(&expected));
    }

    for code in [KeyCode::Tab, KeyCode::BackTab] {
        actions.clear();
        handle_event(
            AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
                code,
                KeyModifiers::SHIFT,
            ))),
            &command_state,
            &config,
            &mut resolver,
            &mut actions,
        );
        assert!(actions.contains(&Action::CompleteCommand { backwards: true }));
    }
}

#[test]
fn confirmation_context_blocks_normal_mappings_and_accepts_only_confirm_keys() {
    let config = test_config();
    let mut state = test_state(&config);
    state.mode = AppMode::Confirm;
    state.dialog_options.display = DialogDisplay::CommandLine;

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();

    for character in ['j', 'y'] {
        let key = crossterm::event::KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        handle_event(
            AppEvent::Input(InputEvent::Key(key)),
            &state,
            &config,
            &mut resolver,
            &mut actions,
        );
    }
    let enter = crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    handle_event(
        AppEvent::Input(InputEvent::Key(enter)),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Action::MoveSelection { .. }))
    );
    assert!(actions.contains(&Action::AcceptConfirmation));
    assert!(!actions.contains(&Action::SubmitCommand));

    for (characters, expected) in [
        ("dD", Action::AcceptConfirmation),
        ("cC", Action::CancelConfirmation),
        ("aA", Action::ToggleFileConflictApplyToAll),
        ("oO", Action::OverwriteFileConflict),
        ("sS", Action::SkipFileConflict),
        ("yY", Action::AcceptConfirmation),
        ("nN", Action::RejectConfirmation),
    ] {
        for character in characters.chars() {
            actions.clear();
            let key = crossterm::event::KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
            handle_event(
                AppEvent::Input(InputEvent::Key(key)),
                &state,
                &config,
                &mut resolver,
                &mut actions,
            );
            assert!(
                actions.contains(&expected),
                "missing confirmation shortcut {character}"
            );
        }
    }
}

#[test]
fn text_input_contexts_use_generic_unicode_printable_fallbacks() {
    let config = test_config();
    let mut state = test_state(&config);

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();
    state.mode = AppMode::Search;
    state.search.active = true;
    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('猫'),
            KeyModifiers::NONE,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert!(actions.contains(&Action::SearchPushCharacter('猫')));

    actions.clear();
    state.mode = AppMode::Normal;
    state.finder.mode = crate::finder::FinderMode::Insert;
    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('界'),
            KeyModifiers::NONE,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert!(actions.contains(&Action::FinderPushCharacter('界')));

    actions.clear();
    state.finder.mode = crate::finder::FinderMode::Closed;
    state.mode = AppMode::Insert;
    state.directory_edit.active = true;
    handle_event(
        AppEvent::Input(InputEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('狼'),
            KeyModifiers::NONE,
        ))),
        &state,
        &config,
        &mut resolver,
        &mut actions,
    );
    assert!(actions.contains(&Action::EditInsertCharacter('狼')));
}

#[test]
fn confirmed_search_uses_normal_keys_until_escape() {
    let config = test_config();
    let mut state = test_state(&config);
    state.mode = AppMode::Search;
    state.search.confirmed_active = true;

    let mut resolver = KeyResolver::default();
    let mut actions = VecDeque::new();

    for (key, expected) in [
        (
            KeyCode::Char('n'),
            Action::SearchRepeat {
                direction: crate::search::SearchDirection::Forward,
                count: 1,
            },
        ),
        (
            KeyCode::Char('N'),
            Action::SearchRepeat {
                direction: crate::search::SearchDirection::Backward,
                count: 1,
            },
        ),
        (KeyCode::Esc, Action::SearchCancel),
    ] {
        actions.clear();
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
        assert!(
            actions.contains(&expected),
            "missing confirmed search key {key:?}"
        );
    }
}
