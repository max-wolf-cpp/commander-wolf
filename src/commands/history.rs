use std::borrow::Cow;

pub const MAX_HISTORY_ENTRIES: usize = 500;
pub const MAX_COMMAND_BYTES: usize = 4096;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandHistory {
    entries: Vec<String>,
    position: Option<usize>,
    draft: String,
}

impl CommandHistory {
    pub fn reset_navigation(&mut self) {
        self.position = None;
        self.draft.clear();
    }

    pub fn record(&mut self, line: &str) {
        self.reset_navigation();

        let line: Cow<'_, str> = if line.starts_with(':') {
            line.into()
        } else {
            format!(":{line}").into()
        };
        if line.strip_prefix(':').unwrap_or(&line).trim().is_empty()
            || line.len() > MAX_COMMAND_BYTES
            || self
                .entries
                .last()
                .is_some_and(|previous| previous == line.as_ref())
        {
            return;
        }

        if self.entries.len() == MAX_HISTORY_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(line.into_owned());
    }

    pub fn previous(&mut self, current: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }

        let position = match self.position {
            Some(position) => position.saturating_sub(1),
            None => {
                self.draft = current.to_owned();
                self.entries.len() - 1
            }
        };
        self.position = Some(position);

        Some(self.entries[position].clone())
    }

    pub fn next_entry(&mut self) -> Option<String> {
        let position = self.position?;
        if position + 1 >= self.entries.len() {
            self.position = None;
            return Some(std::mem::take(&mut self.draft));
        }

        self.position = Some(position + 1);

        Some(self.entries[position + 1].clone())
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_clamps_at_oldest_and_restores_unicode_draft() {
        let mut history = CommandHistory::default();
        assert_eq!(history.previous(":draft"), None);
        assert_eq!(history.next_entry(), None);

        history.record(":cd Source");
        history.record(":style compact");

        assert_eq!(history.previous(":cd 狼"), Some(":style compact".into()));
        assert_eq!(history.previous(":ignored"), Some(":cd Source".into()));
        assert_eq!(history.previous(":ignored"), Some(":cd Source".into()));

        assert_eq!(history.next_entry(), Some(":style compact".into()));
        assert_eq!(history.next_entry(), Some(":cd 狼".into()));
        assert_eq!(history.next_entry(), None);
    }

    #[test]
    fn recording_is_bounded_and_ignores_empty_and_consecutive_duplicates() {
        let mut history = CommandHistory::default();
        for line in ["", ":", ":   ", "  ", ":q", ":q"] {
            history.record(line);
        }
        assert_eq!(history.entries(), &[":q"]);

        history.record(&"x".repeat(MAX_COMMAND_BYTES + 1));
        assert_eq!(history.entries().len(), 1);

        for index in 0..MAX_HISTORY_ENTRIES + 1 {
            history.record(&format!(":cd {index}"));
        }

        assert_eq!(history.entries().len(), MAX_HISTORY_ENTRIES);
        assert_eq!(history.entries()[0], ":cd 1");
    }
}
