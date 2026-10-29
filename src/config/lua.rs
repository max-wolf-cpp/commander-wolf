use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use mlua::Lua;

use crate::commands::{CommandId, CommandRegistry};
use crate::dialogs::DialogOptions;
use crate::file_opening::{Application, FileAssociations};
use crate::finder::FinderOptions;
use crate::input::Keymaps;
use crate::preview::PreviewOptions;
use crate::status_line::StatusLineOptions;
use crate::terminal_appearance::{SchemePreference, TerminalAppearance};
use crate::workspace::WorkspaceOptions;

use super::StandardPaths;

mod api;
mod appearance;
mod applications;
mod commands;
mod components;
mod keymaps;
mod loading;
mod options;

use appearance::{ComponentDefinition, ThemeSpec};

const CALLBACKS_KEY: &str = "commander_wolf.callbacks";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LuaRequest {
    Notify(String),
    Execute {
        command: CommandId,
        args: Vec<String>,
        count: Option<usize>,
    },
    ExecuteLine(String),
}

pub struct ConfigRuntime {
    lua: Lua,
    draft: Rc<RefCell<ConfigDraft>>,
    appearance: Rc<Cell<TerminalAppearance>>,
    pub load_warning: Option<String>,
}

#[derive(Debug)]
struct ConfigDraft {
    leader: String,
    commands: CommandRegistry,
    keymaps: Keymaps,
    requests: Vec<LuaRequest>,
    next_callback: u64,

    themes: HashMap<String, HashMap<String, String>>,
    palette_parents: HashMap<String, String>,
    internal_nearest_palettes: HashSet<String>,
    theme_specs: HashMap<String, ThemeSpec>,
    syntax_themes: HashMap<String, ComponentDefinition>,
    styles: HashMap<String, ComponentDefinition>,
    terminal_appearance: SchemePreference,

    active_theme: Option<String>,
    session_palette: Option<String>,
    session_syntax: Option<Option<String>>,
    session_style: Option<String>,
    session_file_color_policy: Option<crate::theme::DynamicFileColorPolicy>,

    palette_overrides: HashMap<String, String>,
    syntax_overrides: HashMap<String, String>,
    style_overrides: HashMap<String, String>,

    workspace: WorkspaceOptions,
    finder: FinderOptions,
    preview: PreviewOptions,
    status_line: StatusLineOptions,
    dialogs: DialogOptions,
    remember_directories: bool,

    file_associations: FileAssociations,
    shell: Option<Application>,
}

impl Default for ConfigDraft {
    fn default() -> Self {
        Self {
            leader: " ".into(),
            commands: CommandRegistry::with_builtins(),
            keymaps: Keymaps::default(),
            requests: Vec::new(),
            next_callback: 1,

            themes: HashMap::new(),
            palette_parents: HashMap::new(),
            internal_nearest_palettes: HashSet::new(),
            theme_specs: HashMap::new(),
            syntax_themes: HashMap::new(),
            styles: HashMap::new(),
            terminal_appearance: SchemePreference::Auto,

            active_theme: None,
            session_palette: None,
            session_syntax: None,
            session_style: None,
            session_file_color_policy: None,

            palette_overrides: HashMap::new(),
            syntax_overrides: HashMap::new(),
            style_overrides: HashMap::new(),

            workspace: WorkspaceOptions::default(),
            finder: FinderOptions::default(),
            preview: PreviewOptions::default(),
            status_line: StatusLineOptions::default(),
            dialogs: DialogOptions::default(),
            remember_directories: false,

            file_associations: FileAssociations::default(),
            shell: None,
        }
    }
}

fn lua_error(error: mlua::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests;
