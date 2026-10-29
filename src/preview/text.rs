#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextPreview {
    pub lines: Vec<String>,
    pub highlighting: Result<Vec<Vec<crate::syntax::SyntaxSpan>>, String>,
    pub truncated: bool,
}
