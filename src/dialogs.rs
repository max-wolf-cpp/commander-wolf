mod file_conflict;
mod rename;

pub use file_conflict::{FileConflict, FileConflictState};
pub use rename::{RenameInput, RenamePrompt};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogDisplay {
    #[default]
    Float,
    CommandLine,
}

impl DialogDisplay {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "float" => Ok(Self::Float),
            "command_line" => Ok(Self::CommandLine),
            _ => Err(format!(
                "invalid dialog display '{value}'; expected 'float' or 'command_line'"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DialogOptions {
    pub display: DialogDisplay,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialogKind {
    #[default]
    Info,
    Warning,
    Error,
}
