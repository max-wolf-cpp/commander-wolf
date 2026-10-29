use super::{DirectoryEditState, EditLine};

impl DirectoryEditState {
    pub fn insert(&mut self, character: char) {
        let byte = char_byte(&self.lines[self.cursor_row].text, self.cursor_column);
        self.lines[self.cursor_row].text.insert(byte, character);
        self.cursor_column += 1;
    }

    pub fn newline(&mut self) {
        let byte = char_byte(&self.lines[self.cursor_row].text, self.cursor_column);
        let suffix = self.lines[self.cursor_row].text.split_off(byte);

        self.cursor_row += 1;
        self.lines.insert(
            self.cursor_row,
            EditLine {
                text: suffix,
                original: None,
            },
        );

        self.cursor_column = 0;
        self.ensure_visible();
    }

    pub fn backspace(&mut self) {
        if self.cursor_column > 0 {
            let end = char_byte(&self.lines[self.cursor_row].text, self.cursor_column);
            let start = char_byte(&self.lines[self.cursor_row].text, self.cursor_column - 1);
            self.lines[self.cursor_row]
                .text
                .replace_range(start..end, "");
            self.cursor_column -= 1;
        } else if self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_column = self.lines[self.cursor_row].text.chars().count();
            if self.lines[self.cursor_row].original.is_none() {
                self.lines[self.cursor_row].original = current.original;
            }
            self.lines[self.cursor_row].text.push_str(&current.text);
        }

        self.ensure_visible();
    }

    pub fn delete(&mut self) {
        let length = self.lines[self.cursor_row].text.chars().count();

        if self.cursor_column < length {
            let start = char_byte(&self.lines[self.cursor_row].text, self.cursor_column);
            let end = char_byte(&self.lines[self.cursor_row].text, self.cursor_column + 1);
            self.lines[self.cursor_row]
                .text
                .replace_range(start..end, "");
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            if self.lines[self.cursor_row].original.is_none() {
                self.lines[self.cursor_row].original = next.original;
            }
            self.lines[self.cursor_row].text.push_str(&next.text);
        }
    }

    pub fn move_left(&mut self) {
        self.cursor_column = self.cursor_column.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_column = (self.cursor_column + 1).min(self.current_len());
    }

    pub fn move_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(1);
        self.cursor_column = self.cursor_column.min(self.current_len());
        self.ensure_visible();
    }

    pub fn move_down(&mut self) {
        self.cursor_row = (self.cursor_row + 1).min(self.lines.len().saturating_sub(1));
        self.cursor_column = self.cursor_column.min(self.current_len());
        self.ensure_visible();
    }

    pub fn home(&mut self) {
        self.cursor_column = 0;
    }

    pub fn end(&mut self) {
        self.cursor_column = self.current_len();
    }

    fn current_len(&self) -> usize {
        self.lines
            .get(self.cursor_row)
            .map_or(0, |line| line.text.chars().count())
    }
}

fn char_byte(value: &str, column: usize) -> usize {
    value
        .char_indices()
        .nth(column)
        .map_or(value.len(), |(index, _)| index)
}
