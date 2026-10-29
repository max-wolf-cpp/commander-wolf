mod keymaps;
mod keys;
mod resolver;

pub use keymaps::{KeymapBinding, Keymaps};
pub use keys::KeyChord;
pub use resolver::{KeyResolver, ResolveOutcome};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputContext {
    Normal,
    Insert,
    Command,
    Confirm,
    Search,
    Visual,
    FinderInsert,
    FinderSearching,
    FinderNormal,
    Tree,
    Viewer,
}

impl InputContext {
    pub const ALL: [Self; 11] = [
        Self::Normal,
        Self::Visual,
        Self::Insert,
        Self::Command,
        Self::Confirm,
        Self::Search,
        Self::FinderInsert,
        Self::FinderSearching,
        Self::FinderNormal,
        Self::Tree,
        Self::Viewer,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Visual => "Visual",
            Self::Insert => "Insert",
            Self::Command => "Command",
            Self::Confirm => "Confirm",
            Self::Search => "Search",
            Self::FinderInsert => "Finder insert",
            Self::FinderSearching => "Finder search",
            Self::FinderNormal => "Finder normal",
            Self::Tree => "Tree",
            Self::Viewer => "Viewer",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "normal" | "n" => Ok(Self::Normal),
            "insert" | "i" => Ok(Self::Insert),
            "command" | "c" => Ok(Self::Command),
            "confirm" => Ok(Self::Confirm),
            "search" => Ok(Self::Search),
            "visual" | "v" => Ok(Self::Visual),
            "finder_insert" => Ok(Self::FinderInsert),
            "finder_searching" => Ok(Self::FinderSearching),
            "finder_normal" => Ok(Self::FinderNormal),
            "tree" => Ok(Self::Tree),
            "viewer" => Ok(Self::Viewer),
            _ => Err(format!("unknown keymap context: {value}")),
        }
    }
}

#[cfg(test)]
mod tests;
