use std::path::PathBuf;

use super::DialogDisplay;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenameInput {
    Insert(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    SelectAll,
    Clear,
    Submit,
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenamePrompt {
    pub source: PathBuf,
    pub text: String,
    pub cursor: usize,
    pub selected_all: bool,
    pub display: DialogDisplay,
    pub error: Option<String>,
}

impl RenamePrompt {
    pub fn new(source: PathBuf, name: String, display: DialogDisplay) -> Self {
        Self {
            source,
            cursor: name.len(),
            text: name,
            selected_all: false,
            display,
            error: None,
        }
    }

    pub fn edit(&mut self, input: RenameInput) {
        use RenameInput::*;

        if self.selected_all && matches!(input, Insert(_) | Backspace | Delete | Clear) {
            self.text.clear();
            self.cursor = 0;
            self.selected_all = false;
            if matches!(input, Backspace | Delete) {
                self.error = None;
                return;
            }
        }

        match input {
            Insert(character) if !character.is_control() => {
                self.text.insert(self.cursor, character);
                self.cursor += character.len_utf8();
            }
            Backspace if self.cursor > 0 => {
                let start = self.previous_boundary();
                self.text.drain(start..self.cursor);
                self.cursor = start;
            }
            Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
            }
            Left => {
                self.cursor = if self.selected_all {
                    0
                } else {
                    self.previous_boundary()
                }
            }
            Right => {
                self.cursor = if self.selected_all {
                    self.text.len()
                } else {
                    self.next_boundary()
                }
            }
            Home => self.cursor = 0,
            End => self.cursor = self.text.len(),
            Clear => {
                self.text.clear();
                self.cursor = 0;
            }
            SelectAll => {
                self.selected_all = true;
                return;
            }
            _ => {}
        }

        self.selected_all = false;
        self.error = None;
    }

    fn previous_boundary(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index)
    }

    fn next_boundary(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_input_edits_unicode_and_replaces_selection() {
        use RenameInput::*;

        let mut prompt = RenamePrompt::new("猫.txt".into(), "猫.txt".into(), DialogDisplay::Float);
        prompt.edit(Home);
        prompt.edit(Right);
        assert_eq!(prompt.cursor, "猫".len());
        prompt.edit(Backspace);
        assert_eq!(prompt.text, ".txt");
        prompt.edit(Insert('狼'));
        prompt.edit(Delete);
        assert_eq!(prompt.text, "狼txt");

        prompt.edit(SelectAll);
        prompt.edit(Insert('新'));
        assert_eq!(prompt.text, "新");
        prompt.edit(Left);
        prompt.edit(Insert('a'));
        prompt.edit(End);
        prompt.edit(Backspace);
        assert_eq!(prompt.text, "a");

        prompt.edit(SelectAll);
        prompt.edit(Delete);
        assert!(prompt.text.is_empty());
        assert_eq!(prompt.cursor, 0);
    }
}
