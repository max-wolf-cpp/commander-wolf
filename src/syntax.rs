use std::ops::Range;
use std::path::Path;
use std::sync::OnceLock;

pub use palette::SyntaxTheme;
pub use syntect::SyntectHighlighter;

mod palette;
mod syntect;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxCategory {
    Text,
    Comment,
    Keyword,
    String,
    Escape,
    Number,
    Boolean,
    Type,
    Function,
    Variable,
    Parameter,
    Property,
    Constant,
    Operator,
    Punctuation,
    Tag,
    Attribute,
    Preprocessor,
    Heading,
    Link,
    Code,
    Emphasis,
    Strong,
    Added,
    Removed,
    Invalid,
}

impl SyntaxCategory {
    pub const ALL: [Self; 26] = [
        Self::Text,
        Self::Comment,
        Self::Keyword,
        Self::String,
        Self::Escape,
        Self::Number,
        Self::Boolean,
        Self::Type,
        Self::Function,
        Self::Variable,
        Self::Parameter,
        Self::Property,
        Self::Constant,
        Self::Operator,
        Self::Punctuation,
        Self::Tag,
        Self::Attribute,
        Self::Preprocessor,
        Self::Heading,
        Self::Link,
        Self::Code,
        Self::Emphasis,
        Self::Strong,
        Self::Added,
        Self::Removed,
        Self::Invalid,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Comment => "comment",
            Self::Keyword => "keyword",
            Self::String => "string",
            Self::Escape => "escape",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Type => "type",
            Self::Function => "function",
            Self::Variable => "variable",
            Self::Parameter => "parameter",
            Self::Property => "property",
            Self::Constant => "constant",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::Tag => "tag",
            Self::Attribute => "attribute",
            Self::Preprocessor => "preprocessor",
            Self::Heading => "heading",
            Self::Link => "link",
            Self::Code => "code",
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxSpan {
    /// UTF-8 byte offsets into one unchanged preview line.
    pub range: Range<usize>,
    pub category: SyntaxCategory,
}

/// Backends return ordered, non-overlapping spans without terminal or palette types.
/// Missing lines or gaps are plain text. Implementations may bound their work.
pub trait SyntaxHighlighter: Send + Sync {
    fn highlight(&self, path: &Path, lines: &[String]) -> Result<Vec<Vec<SyntaxSpan>>, String>;
}

pub fn default_highlighter() -> &'static dyn SyntaxHighlighter {
    static HIGHLIGHTER: OnceLock<SyntectHighlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(SyntectHighlighter::new)
}
