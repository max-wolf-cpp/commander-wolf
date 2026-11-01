use fox_media_term::{Environment, Protocol};

use crate::cli::Arguments;
use crate::config::{ConfigRuntime, StandardPaths};

use self::document::DocumentRuntime;
use self::finder::FinderRuntime;
use self::media::MediaRuntime;
use self::terminal_writer::TerminalWriter;

mod actions;
mod clipboard;
mod command_history;
mod document;
mod effects;
mod event_loop;
mod finder;
mod input;
mod media;
mod operations;
mod process_tree;
mod session;
mod shell;
mod state_file;
mod terminal_input;
mod terminal_writer;
mod tree;
mod watcher;

struct Persistence {
    session: session::SessionStore,
    history: command_history::HistoryStore,
}

struct RuntimeResources<'a> {
    clipboard: &'a mut clipboard::ClipboardRuntime,
    media: &'a mut MediaRuntime,
    finder: &'a mut FinderRuntime,
    documents: &'a mut DocumentRuntime,
    shell: &'a mut shell::ShellRuntime,
    operations: &'a mut operations::OperationsRuntime,
    tree: &'a mut tree::TreeRuntime,
}

pub fn run() -> Result<(), String> {
    let arguments = Arguments::parse_env()?;
    let paths = StandardPaths::discover()?;
    let persistence = Persistence {
        session: session::SessionStore::new(paths.state.clone()),
        history: command_history::HistoryStore::new(paths.state.clone(), !arguments.clean),
    };
    let config = ConfigRuntime::load(paths, arguments.clean)?;

    let (columns, rows) = crossterm::terminal::size()
        .map_err(|error| format!("cannot read terminal size: {error}"))?;
    // Native capability/geometry queries own the console only before the TUI
    // starts. During the session, TerminalInput owns replies and keyboard input.
    let detected = Environment::detect(i32::from(columns), i32::from(rows))
        .map_err(|error| error.to_string())?;
    let environment = Environment {
        protocol: if arguments.protocol == Protocol::Auto {
            detected.protocol
        } else {
            arguments.protocol
        },
        ..detected
    };

    let media = MediaRuntime::new(environment);
    let protocol_label = media.protocol_label();
    let mut writer = TerminalWriter::initialize();
    let theme = config.theme()?;

    let result = event_loop::run_event_loop(
        arguments,
        config,
        theme,
        media,
        &mut writer,
        protocol_label,
        persistence,
    );

    drop(writer);
    ratatui::restore();

    result
}
