use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyEventKind};

use crate::app::{Action, AppEvent, AppState, InputEvent, Notification};
use crate::cli::Arguments;
use crate::config::ConfigRuntime;
use crate::input::KeyResolver;
use crate::terminal::TerminalSize;
use crate::terminal_appearance::TerminalAppearance;
use crate::theme::Theme;

use super::actions::process_actions;
use super::document::DocumentRuntime;
use super::finder::FinderRuntime;
use super::input::handle_event;
use super::media::MediaRuntime;
use super::terminal_input::TerminalInput;
use super::terminal_writer::TerminalWriter;
use super::{Persistence, RuntimeResources, clipboard, effects, operations, shell, tree, watcher};

pub(super) fn run_event_loop(
    arguments: Arguments,
    config: ConfigRuntime,
    mut theme: Theme,
    mut media: MediaRuntime,
    writer: &mut TerminalWriter,
    protocol_label: String,
    persistence: Persistence,
) -> Result<(), String> {
    let terminal_size = writer.size()?;
    let (mut state, mut startup_actions) = startup_state(
        arguments,
        &config,
        &persistence,
        terminal_size,
        protocol_label,
    )?;

    let mut resolver = KeyResolver::default();
    let mut clipboard_runtime = clipboard::ClipboardRuntime::default();
    let mut finder_runtime = FinderRuntime::new();
    let mut document_runtime = DocumentRuntime::new();
    let mut shell_runtime = shell::ShellRuntime::default();
    let mut operations_runtime = operations::OperationsRuntime::default();
    let mut tree_runtime = tree::TreeRuntime::new();
    let mut directory_watcher = match watcher::DirectoryWatcher::new() {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            startup_actions.push(Action::Notify(Notification::error(format!(
                "{error}; use :refresh to update panels"
            ))));
            None
        }
    };

    let mut actions = VecDeque::from(startup_actions);
    match effects::dispatch_requests(config.take_requests(), &config) {
        Ok(requested) => actions.extend(requested),
        Err(error) => actions.push_back(Action::Notify(Notification::error(error))),
    }

    let mut ui_dirty = true;
    let mut terminal_input = TerminalInput::default();
    let mut last_appearance_probe: Option<Instant> = None;
    let mut appearance_recheck_needed = true;

    while !state.should_quit {
        process_actions(
            &mut actions,
            &mut state,
            &config,
            RuntimeResources {
                clipboard: &mut clipboard_runtime,
                media: &mut media,
                finder: &mut finder_runtime,
                documents: &mut document_runtime,
                shell: &mut shell_runtime,
                operations: &mut operations_runtime,
                tree: &mut tree_runtime,
            },
            writer,
            &mut theme,
            &mut ui_dirty,
        )?;

        if let Some(appearance) = terminal_input.take_appearance() {
            ui_dirty |= refresh_terminal_appearance(appearance, &config, &state, &mut theme)?;
        }

        appearance_recheck_needed |= writer.take_resumed();
        if let Some(watcher) = &mut directory_watcher {
            if let Some(action) = watcher.synchronize(&state) {
                actions.push_back(action);
            }
            if let Some(action) = watcher.poll(&state) {
                actions.push_back(action);
            }
        }

        if ui_dirty {
            writer.draw(&state, &theme)?;
            ui_dirty = false;
        }

        let appearance_query_due =
            last_appearance_probe.is_none_or(|last| last.elapsed() >= Duration::from_secs(10));
        let can_query_appearance =
            last_appearance_probe.is_none_or(|last| last.elapsed() >= Duration::from_millis(500));
        if (appearance_recheck_needed || appearance_query_due) && can_query_appearance {
            terminal_input.begin_appearance_query();
            writer.write_query(&crate::terminal_appearance::color_query())?;
            last_appearance_probe = Some(Instant::now());
            appearance_recheck_needed = false;
        }

        if let Some(action) = media.poll(state.terminal_size, writer) {
            actions.push_back(action);
            continue;
        }

        actions.extend(finder_runtime.poll());
        actions.extend(document_runtime.poll());
        actions.extend(shell_runtime.poll());
        actions.extend(operations_runtime.poll());
        tree_runtime.synchronize(state.directory_tree.open, state.directory_tree.generation);
        actions.extend(tree_runtime.poll());

        // A continuously writing child must not starve keyboard input.
        let input_wait = if actions.is_empty() {
            Duration::from_millis(5)
        } else {
            Duration::ZERO
        };
        let Some(input) = terminal_input.poll(input_wait)? else {
            continue;
        };

        let event = match input {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                Some(AppEvent::Input(InputEvent::Key(key)))
            }
            Event::Resize(columns, rows) => {
                Some(AppEvent::Input(InputEvent::Resize { columns, rows }))
            }
            Event::FocusGained => {
                appearance_recheck_needed = true;
                None
            }
            Event::Mouse(_) => None,
            _ => None,
        };
        if let Some(event) = event {
            handle_event(event, &state, &config, &mut resolver, &mut actions);
        }
    }

    let saved_session = persistence
        .session
        .save(config.remember_directories(), &state.workspace);
    let saved_history = persistence.history.save(&state.command_history);

    media.clear_current(&theme, writer)?;
    finder_runtime.shutdown();
    document_runtime.shutdown();

    match (saved_session, saved_history) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(session), Err(history)) => Err(format!("{session}; {history}")),
    }
}

fn startup_state(
    arguments: Arguments,
    config: &ConfigRuntime,
    persistence: &Persistence,
    terminal_size: TerminalSize,
    protocol_label: String,
) -> Result<(AppState, Vec<Action>), String> {
    let mut startup_actions = persistence.session.startup_actions(
        &arguments.path,
        config.remember_directories() && !arguments.explicit_path,
    );
    let mut state = AppState::new(
        arguments.path,
        protocol_label,
        terminal_size,
        config.workspace_options(),
    );

    match persistence.history.load() {
        Ok(history) => state.command_history = history,
        Err(error) => startup_actions.push(Action::Notify(Notification::error(error))),
    }

    state.finder_options = config.finder_options();
    state.preview_options = config.preview_options();
    state.syntax_theme = config.syntax_theme()?;
    state.syntax_enabled = config.syntax_enabled();
    state.status_line_options = config.status_line_options();
    state.dialog_options = config.dialog_options();
    state.file_associations = config.file_associations();

    let panel_rows = usize::from(
        crate::ui::resolve_layout_with_styles(
            terminal_size.area(),
            None,
            state.preview_options.media_percent,
            state.status_line_options.style,
            state.workspace.panel_style,
        )
        .left_inner
        .height,
    );
    state.workspace.set_visible_rows(panel_rows);
    if let Some(warning) = config.load_warning.clone() {
        state.notification = Some(Notification::error(warning));
    }

    Ok((state, startup_actions))
}

fn refresh_terminal_appearance(
    appearance: TerminalAppearance,
    config: &ConfigRuntime,
    state: &AppState,
    theme: &mut Theme,
) -> Result<bool, String> {
    if !config.update_terminal_appearance(appearance) {
        return Ok(false);
    }

    *theme = config.theme()?;
    if let Some(style) = state.separator_style_override {
        theme.separator_style = style;
    }
    if let Some(style) = state.border_style_override {
        theme.set_border_style(style);
    }

    Ok(true)
}
