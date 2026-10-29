use std::path::Path;

use syntect::highlighting::ScopeSelectors;
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use super::{SyntaxCategory, SyntaxHighlighter, SyntaxSpan};

mod scope_rules;

const MAX_LINES: usize = 1024;
const MAX_BYTES: usize = 128 * 1024;
const MAX_LINE_BYTES: usize = 4096;

pub struct SyntectHighlighter {
    syntaxes: SyntaxSet,
    rules: Vec<(ScopeSelectors, SyntaxCategory)>,
}

impl SyntectHighlighter {
    pub fn new() -> Self {
        let rules = scope_rules::load();

        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            rules,
        }
    }
}

impl Default for SyntectHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter for SyntectHighlighter {
    fn highlight(&self, path: &Path, lines: &[String]) -> Result<Vec<Vec<SyntaxSpan>>, String> {
        let Some(syntax) = self.find_syntax(path, lines) else {
            return Ok(Vec::new());
        };

        if syntax.scope == self.syntaxes.find_syntax_plain_text().scope {
            return Ok(Vec::new());
        }

        let mut parser = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut output = Vec::new();
        let mut bytes = 0usize;
        for line in lines.iter().take(MAX_LINES) {
            bytes = bytes.saturating_add(line.len());
            // Stop instead of skipping lines: multiline scopes must remain coherent.
            if bytes > MAX_BYTES || line.len() > MAX_LINE_BYTES {
                break;
            }

            output.push(self.highlight_line(line, &mut parser, &mut stack)?);
        }

        Ok(output)
    }
}

impl SyntectHighlighter {
    fn find_syntax(&self, path: &Path, lines: &[String]) -> Option<&SyntaxReference> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        self.syntaxes
            .find_syntax_by_extension(name)
            .or_else(|| self.syntaxes.find_syntax_by_extension(&extension))
            .or_else(|| {
                lines
                    .first()
                    .and_then(|line| self.syntaxes.find_syntax_by_first_line(line))
            })
    }

    fn highlight_line(
        &self,
        line: &str,
        parser: &mut ParseState,
        stack: &mut ScopeStack,
    ) -> Result<Vec<SyntaxSpan>, String> {
        let input = format!("{line}\n");
        let operations = parser
            .parse_line(&input, &self.syntaxes)
            .map_err(|error| error.to_string())?;

        let mut spans: Vec<SyntaxSpan> = Vec::new();
        let mut start = 0;
        for (offset, operation) in operations {
            let end = offset.min(line.len());
            if start < end {
                push_span(
                    &mut spans,
                    start..end,
                    scope_rules::category(&self.rules, stack),
                );
            }
            stack.apply(&operation).map_err(|error| error.to_string())?;
            start = end;
        }

        if start < line.len() {
            push_span(
                &mut spans,
                start..line.len(),
                scope_rules::category(&self.rules, stack),
            );
        }

        Ok(spans)
    }
}

fn push_span(spans: &mut Vec<SyntaxSpan>, range: std::ops::Range<usize>, category: SyntaxCategory) {
    if let Some(last) = spans.last_mut()
        && last.category == category
        && last.range.end == range.start
    {
        last.range.end = range.end;
    } else {
        spans.push(SyntaxSpan { range, category });
    }
}

#[cfg(test)]
mod tests;
