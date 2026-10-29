pub mod app;
pub mod cli;
pub mod commands;
pub mod config;
pub mod dialogs;
pub mod directory_edit;
pub mod directory_tree;
pub mod display_style;
pub mod file_opening;
pub mod file_operations;
pub mod filesystem;
pub mod finder;
pub mod help;
pub mod icons;
pub mod input;
mod palette_colors;
pub mod preview;
pub mod runtime;
pub mod search;
pub mod shell;
pub mod status_line;
pub mod syntax;
pub mod terminal;
pub mod terminal_appearance;
#[cfg(test)]
mod test_support;
pub mod theme;
pub mod ui;
pub mod workspace;

pub fn run() -> Result<(), String> {
    runtime::run()
}
