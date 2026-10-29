#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLine {
    pub name: String,
    pub args: Vec<String>,
}

pub fn parse_command_line(source: &str) -> Result<CommandLine, String> {
    let source = source.strip_prefix(':').unwrap_or(source).trim_start();
    if let Some(command) = source.strip_prefix('!') {
        return Ok(CommandLine {
            name: "!".into(),
            args: vec![command.trim_start().to_owned()],
        });
    }

    let words = parse_words(source.trim_end())?;
    let mut words = words.into_iter();
    let name = words.next().ok_or("empty command")?;

    Ok(CommandLine {
        name,
        args: words.collect(),
    })
}

fn parse_words(source: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in source.chars() {
        if escaped {
            if character.is_whitespace() || matches!(character, '\\' | '\'' | '"') {
                current.push(character);
            } else {
                current.push('\\');
                current.push(character);
            }
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if let Some(expected) = quote {
            if character == expected {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            character if character.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }

    if escaped {
        current.push('\\');
    }
    if quote.is_some() {
        return Err("unterminated quote in command".to_owned());
    }

    if !current.is_empty() {
        words.push(current);
    }

    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_commands_keep_shell_syntax_verbatim() {
        let command = "printf '%s' \"a b\" | tool > out; echo $value ";
        assert_eq!(
            parse_command_line(&format!(":!{command}")).unwrap(),
            CommandLine {
                name: "!".into(),
                args: vec![command.into()],
            }
        );
        assert_eq!(
            parse_command_line(":!Remove-Item ./test").unwrap().args,
            ["Remove-Item ./test"]
        );
    }

    #[test]
    fn parses_quotes_and_escapes() {
        assert_eq!(
            parse_command_line(":open \"a b\" c\\ d").unwrap(),
            CommandLine {
                name: "open".into(),
                args: vec!["a b".into(), "c d".into()]
            }
        );
        assert_eq!(
            parse_command_line(r#":cd C:\目录\file name"#).unwrap().args,
            vec![r#"C:\目录\file"#, "name"]
        );
    }

    #[test]
    fn rejects_empty_commands_and_unterminated_quotes() {
        for source in ["", ":", ":  ", ":''", ":\"\""] {
            assert_eq!(parse_command_line(source).unwrap_err(), "empty command");
        }

        for source in [":open 'path", ":open \"path", ":open 'path\\'"] {
            assert_eq!(
                parse_command_line(source).unwrap_err(),
                "unterminated quote in command"
            );
        }
    }

    #[test]
    fn preserves_literal_backslashes_and_ignores_empty_quoted_words() {
        assert_eq!(
            parse_command_line(r#":open '' "" C:\path "a\q" a\'b a\"b a\\b trailing\"#)
                .unwrap()
                .args,
            [r"C:\path", r"a\q", "a'b", "a\"b", "a\\b", "trailing\\"]
        );
    }
}
