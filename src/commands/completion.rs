#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionItem {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandCompletion {
    pub items: Vec<CompletionItem>,
    pub selected: usize,
    pub original: String,
}

impl CommandCompletion {
    pub fn new(original: String, items: Vec<CompletionItem>, backwards: bool) -> Option<Self> {
        if items.is_empty() {
            return None;
        }

        let selected = if backwards { items.len() - 1 } else { 0 };

        Some(Self {
            items,
            selected,
            original,
        })
    }

    pub fn cycle(&mut self, backwards: bool) {
        self.selected = if backwards {
            (self.selected + self.items.len() - 1) % self.items.len()
        } else {
            (self.selected + 1) % self.items.len()
        };
    }

    pub fn line(&self) -> String {
        let prefix = command_prefix(&self.original).unwrap_or_else(|| {
            self.original
                .rsplit_once(char::is_whitespace)
                .map_or("", |(_, prefix)| prefix)
        });
        let leading = &self.original[..self.original.len() - prefix.len()];

        format!("{leading}{}", self.items[self.selected].name)
    }

    pub fn is_argument(&self) -> bool {
        command_prefix(&self.original).is_none()
    }
}

// The editor currently appends at the end.
pub fn command_prefix(line: &str) -> Option<&str> {
    let prefix = line.strip_prefix(':').unwrap_or(line).trim_start();
    (!prefix
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '\\')))
    .then_some(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_cycles_both_ways_without_losing_the_original_prefix() {
        let items = ["copy", "count"]
            .map(|name| CompletionItem {
                name: name.into(),
                description: String::new(),
            })
            .to_vec();

        let mut completion = CommandCompletion::new(":  co".into(), items.clone(), false).unwrap();
        assert_eq!(completion.line(), ":  copy");

        completion.cycle(false);
        assert_eq!(completion.line(), ":  count");

        completion.cycle(false);
        assert_eq!(completion.line(), ":  copy");

        completion.cycle(true);
        assert_eq!(completion.line(), ":  count");
        assert_eq!(completion.original, ":  co");

        assert_eq!(
            CommandCompletion::new(":".into(), items, true)
                .unwrap()
                .line(),
            ":count"
        );

        assert!(CommandCompletion::new(":".into(), vec![], false).is_none());
    }

    #[test]
    fn only_the_command_name_is_completed() {
        for line in [":", ":  ", ""] {
            assert_eq!(command_prefix(line), Some(""));
        }

        assert_eq!(command_prefix(":ф"), Some("ф"));

        for line in [":cd ", ":cd path", ":'co", ":co\\"] {
            assert_eq!(command_prefix(line), None);
        }
    }

    #[test]
    fn argument_completion_replaces_only_the_last_token() {
        let items = ["nerd_font", "none"]
            .map(|name| CompletionItem {
                name: name.into(),
                description: String::new(),
            })
            .to_vec();

        let mut completion = CommandCompletion::new(":icons n".into(), items, false).unwrap();
        assert!(completion.is_argument());
        assert_eq!(completion.line(), ":icons nerd_font");

        completion.cycle(false);
        assert_eq!(completion.line(), ":icons none");
        assert_eq!(completion.original, ":icons n");

        let completion = CommandCompletion::new(
            ":style panels ".into(),
            vec![CompletionItem {
                name: "compact".into(),
                description: String::new(),
            }],
            false,
        )
        .unwrap();
        assert_eq!(completion.line(), ":style panels compact");
    }
}
