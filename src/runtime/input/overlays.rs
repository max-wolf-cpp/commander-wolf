use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, AppMode, AppState};
use crate::dialogs::DialogDisplay;
use crate::input::KeyResolver;

pub(super) fn handle_key(
    key: KeyEvent,
    state: &AppState,
    resolver: &mut KeyResolver,
    actions: &mut VecDeque<Action>,
) -> bool {
    if state.rename_prompt.is_some() {
        resolver.reset();
        let input = rename_action(key);
        if let Some(input) = input {
            actions.push_back(Action::RenameInput(input));
        }
        return true;
    }

    if state.operation_progress.is_some() {
        resolver.reset();
        if matches!(key.code, KeyCode::Esc)
            || matches!(key.code, KeyCode::Char('c' | 'C'))
                && !key.modifiers.contains(KeyModifiers::ALT)
        {
            actions.push_back(Action::CancelFileOperation);
        }
        return true;
    }

    if state.help.is_some() {
        resolver.reset();
        let page = crate::ui::help_visible_rows(state.terminal_size.area()).max(1) as isize;
        let action = help_action(key, page);
        if let Some(action) = action {
            actions.push_back(action);
        }
        return true;
    }

    if let Some(output) = &state.shell_output {
        resolver.reset();
        let page = (state.terminal_size.rows / 2).saturating_sub(5).max(1) as isize;
        let action = shell_action(key, page, output.finished.is_some(), actions);
        if let Some(action) = action {
            actions.push_back(action);
        }
        return true;
    }

    if state.mode == AppMode::Confirm
        && state.dialog_options.display == DialogDisplay::Float
        && let Some(conflict) = &state.file_conflict
    {
        resolver.reset();
        actions.push_back(Action::SetPendingKeys(String::new()));
        if let Some(action) = file_conflict_float_action(key, conflict.is_batch()) {
            actions.push_back(action);
        }
        return true;
    }

    if state.mode == AppMode::Confirm
        && state.dialog_options.display == DialogDisplay::Float
        && state.directory_edit.awaiting_confirmation
    {
        resolver.reset();
        actions.push_back(Action::SetPendingKeys(String::new()));
        if let Some(action) = directory_edit_float_action(key) {
            actions.push_back(action);
        }
        return true;
    }

    if state.mode == AppMode::Confirm
        && state.dialog_options.display == DialogDisplay::Float
        && state
            .confirmation
            .as_ref()
            .is_some_and(|request| request.is_delete_confirmation())
    {
        resolver.reset();
        actions.push_back(Action::SetPendingKeys(String::new()));
        if let Some(action) = delete_float_action(key) {
            actions.push_back(action);
        }
        return true;
    }

    false
}

fn rename_action(key: KeyEvent) -> Option<crate::dialogs::RenameInput> {
    use crate::dialogs::RenameInput;

    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => Some(RenameInput::Cancel),
        (KeyCode::Enter, _) => Some(RenameInput::Submit),
        (KeyCode::Char('c' | 'C'), KeyModifiers::CONTROL) => Some(RenameInput::Cancel),
        (KeyCode::Char('a' | 'A'), KeyModifiers::CONTROL) => Some(RenameInput::SelectAll),
        (KeyCode::Char('u' | 'U'), KeyModifiers::CONTROL) => Some(RenameInput::Clear),
        (KeyCode::Backspace, _) => Some(RenameInput::Backspace),
        (KeyCode::Delete, _) => Some(RenameInput::Delete),
        (KeyCode::Left, _) => Some(RenameInput::Left),
        (KeyCode::Right, _) => Some(RenameInput::Right),
        (KeyCode::Home, _) => Some(RenameInput::Home),
        (KeyCode::End, _) => Some(RenameInput::End),
        (KeyCode::Char(character), modifiers)
            if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(RenameInput::Insert(character))
        }
        _ => None,
    }
}

fn help_action(key: KeyEvent, page: isize) -> Option<Action> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | '?') => Some(Action::CloseHelp),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollHelp(-1)),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollHelp(1)),
        KeyCode::PageUp => Some(Action::ScrollHelp(-page)),
        KeyCode::PageDown => Some(Action::ScrollHelp(page)),
        KeyCode::Home => Some(Action::ScrollHelp(isize::MIN)),
        KeyCode::End => Some(Action::ScrollHelp(isize::MAX)),
        _ => None,
    }
}

fn shell_action(
    key: KeyEvent,
    page: isize,
    finished: bool,
    actions: &mut VecDeque<Action>,
) -> Option<Action> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::CancelShellCommand)
        }
        KeyCode::Up => Some(Action::ScrollShellOutput(1)),
        KeyCode::Down => Some(Action::ScrollShellOutput(-1)),
        KeyCode::PageUp => Some(Action::ScrollShellOutput(page)),
        KeyCode::PageDown => Some(Action::ScrollShellOutput(-page)),
        KeyCode::Home => Some(Action::ScrollShellOutput(isize::MAX)),
        KeyCode::End => Some(Action::ScrollShellOutput(isize::MIN)),
        KeyCode::Enter | KeyCode::Esc if finished => Some(Action::CloseShellOutput),
        KeyCode::Char(':') if finished => {
            actions.push_back(Action::CloseShellOutput);
            Some(Action::EnterCommandMode)
        }
        _ => None,
    }
}

pub(super) fn file_conflict_float_action(
    key: crossterm::event::KeyEvent,
    is_batch: bool,
) -> Option<Action> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }

    match key.code {
        KeyCode::Char(character) => match character.to_ascii_lowercase() {
            'a' if is_batch => Some(Action::ToggleFileConflictApplyToAll),
            'o' => Some(Action::OverwriteFileConflict),
            's' => Some(Action::SkipFileConflict),
            'c' => Some(Action::CancelConfirmation),
            _ => None,
        },
        KeyCode::Esc => Some(Action::CancelConfirmation),
        _ => None,
    }
}

pub(super) fn directory_edit_float_action(key: crossterm::event::KeyEvent) -> Option<Action> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }

    match key.code {
        KeyCode::Char(character) => match character.to_ascii_lowercase() {
            'y' => Some(Action::AcceptConfirmation),
            'n' => Some(Action::RejectConfirmation),
            'c' => Some(Action::CancelConfirmation),
            _ => None,
        },
        KeyCode::Esc => Some(Action::CancelConfirmation),
        _ => None,
    }
}

pub(super) fn delete_float_action(key: crossterm::event::KeyEvent) -> Option<Action> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }

    match key.code {
        KeyCode::Char(character) => match character.to_ascii_lowercase() {
            'd' => Some(Action::AcceptConfirmation),
            'c' => Some(Action::CancelConfirmation),
            _ => None,
        },
        KeyCode::Esc => Some(Action::CancelConfirmation),
        _ => None,
    }
}
