use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, AppEvent, AppMode, AppState, InputEvent, Notification};
use crate::config::ConfigRuntime;
use crate::input::{InputContext, KeyChord, KeyResolver, ResolveOutcome};
use crate::terminal::TerminalSize;

use super::effects;

mod overlays;

pub(super) fn handle_event(
    event: AppEvent,
    state: &AppState,
    config: &ConfigRuntime,
    resolver: &mut KeyResolver,
    actions: &mut VecDeque<Action>,
) {
    match event {
        AppEvent::Action(action) => actions.push_back(*action),
        AppEvent::Input(InputEvent::Resize { columns, rows }) => {
            handle_resize(TerminalSize { columns, rows }, state, resolver, actions);
        }
        AppEvent::Input(InputEvent::Key(key)) => {
            if overlays::handle_key(key, state, resolver, actions) {
                return;
            }

            handle_mapped_key(key, state, config, resolver, actions);
        }
        AppEvent::VideoTick => {}
    }
}

fn handle_resize(
    size: TerminalSize,
    state: &AppState,
    resolver: &mut KeyResolver,
    actions: &mut VecDeque<Action>,
) {
    resolver.reset();
    actions.push_back(Action::SetPendingKeys(String::new()));

    let panel_rows = usize::from(
        crate::ui::resolve_layout_with_styles(
            size.area(),
            state.workspace.preview_panel(),
            state.preview_options.media_percent,
            state.status_line_options.style,
            state.workspace.panel_style,
        )
        .left_inner
        .height,
    );
    let finder_rows = crate::ui::finder_visible_rows(size.area());
    let tree_rows = crate::ui::tree_visible_rows(size.area());

    actions.push_back(Action::Resize {
        size,
        panel_rows,
        finder_rows,
        tree_rows,
    });
}

fn handle_mapped_key(
    key: KeyEvent,
    state: &AppState,
    config: &ConfigRuntime,
    resolver: &mut KeyResolver,
    actions: &mut VecDeque<Action>,
) {
    let context = key_context(state);
    let outcome = {
        let maps = config.keymaps();
        resolver.resolve(&maps, context, KeyChord::from(key))
    };

    match outcome {
        ResolveOutcome::Pending(display) => actions.push_back(Action::SetPendingKeys(display)),
        ResolveOutcome::Command(invocation) => {
            actions.push_back(Action::SetPendingKeys(String::new()));
            match config
                .dispatch(&invocation.id, invocation.arguments, invocation.count)
                .and_then(|dispatch| effects::dispatch_command(dispatch, config))
            {
                Ok(next) => actions.extend(next),
                Err(error) => actions.push_back(Action::Notify(Notification::error(error))),
            }
        }
        ResolveOutcome::Unmapped {
            chord,
            consumed_prefix,
        } => {
            actions.push_back(Action::SetPendingKeys(String::new()));
            if matches!(
                context,
                InputContext::Command
                    | InputContext::Search
                    | InputContext::FinderInsert
                    | InputContext::Insert
            ) && !consumed_prefix
                && !chord
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && let KeyCode::Char(character) = chord.code
            {
                actions.push_back(match context {
                    InputContext::Command => Action::PushCommandCharacter(character),
                    InputContext::Search => Action::SearchPushCharacter(character),
                    InputContext::FinderInsert => Action::FinderPushCharacter(character),
                    InputContext::Insert => Action::EditInsertCharacter(character),
                    _ => unreachable!("printable fallback context was checked"),
                });
            }
        }
    }
}

fn key_context(state: &AppState) -> InputContext {
    if state.viewer.is_open() {
        return InputContext::Viewer;
    }

    if state.directory_tree.open {
        return InputContext::Tree;
    }

    match state.finder.mode {
        crate::finder::FinderMode::Insert => InputContext::FinderInsert,
        crate::finder::FinderMode::Searching => InputContext::FinderSearching,
        crate::finder::FinderMode::Normal => InputContext::FinderNormal,
        crate::finder::FinderMode::Closed => match state.mode {
            AppMode::Normal => InputContext::Normal,
            AppMode::Insert => InputContext::Insert,
            AppMode::Command => InputContext::Command,
            AppMode::Confirm => InputContext::Confirm,
            AppMode::Search if state.search.active => InputContext::Search,
            AppMode::Search => InputContext::Normal,
            AppMode::Visual => InputContext::Visual,
        },
    }
}

#[cfg(test)]
mod tests;
