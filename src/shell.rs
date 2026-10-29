use std::collections::VecDeque;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_OUTPUT_LINES: usize = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellOutput {
    pub command: String,
    pub lines: VecDeque<String>,
    pub finished: Option<String>,
    pub failed: bool,
    pub scroll: usize,
    pub truncated: bool,
    bytes: usize,
}

impl ShellOutput {
    pub fn new(command: String) -> Self {
        Self {
            command,
            lines: VecDeque::from([String::new()]),
            finished: None,
            failed: false,
            scroll: 0,
            truncated: false,
            bytes: 0,
        }
    }

    pub fn append(&mut self, text: &str) {
        for character in text.chars() {
            let bytes = match character {
                '\t' => 4,
                '\n' => 1,
                character if !character.is_control() => character.len_utf8(),
                _ => 0,
            };

            if self.bytes.saturating_add(bytes) > MAX_OUTPUT_BYTES
                || self.lines.len() >= MAX_OUTPUT_LINES
            {
                self.truncated = true;
                break;
            }

            self.push_character(character);
            self.bytes += bytes;
        }
    }

    fn push_character(&mut self, character: char) {
        match character {
            '\n' => self.lines.push_back(String::new()),
            '\r' => {}
            '\t' => self.lines.back_mut().unwrap().push_str("    "),
            character if !character.is_control() => {
                self.lines.back_mut().unwrap().push(character);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_bounded_and_control_characters_are_not_rendered() {
        let mut output = ShellOutput::new("echo".into());
        output.append("hello\r\n狼\tworld\x07");
        assert_eq!(output.lines[0], "hello");
        assert_eq!(output.lines[1], "狼    world");

        output.append(&"x".repeat(MAX_OUTPUT_BYTES * 2));
        assert!(output.truncated);
        assert!(output.bytes <= MAX_OUTPUT_BYTES);
    }

    #[test]
    fn byte_limit_never_splits_unicode_or_overflows_on_tab_expansion() {
        for suffix in ["🙂", "狼", "\t"] {
            let mut output = ShellOutput::new("fixture".into());
            output.append(&"x".repeat(MAX_OUTPUT_BYTES - 1));
            output.append(suffix);

            assert!(output.truncated);
            assert_eq!(output.bytes, MAX_OUTPUT_BYTES - 1);
            assert_eq!(output.lines[0].len(), MAX_OUTPUT_BYTES - 1);
        }
    }
}
