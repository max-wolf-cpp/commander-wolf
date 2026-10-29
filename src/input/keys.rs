use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl From<KeyEvent> for KeyChord {
    fn from(event: KeyEvent) -> Self {
        let mut modifiers = event.modifiers;
        if matches!(event.code, KeyCode::Char(_)) {
            modifiers.remove(KeyModifiers::SHIFT);
        }

        // BackTab may arrive with or without SHIFT, depending on the terminal.
        let code = if event.code == KeyCode::BackTab {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::Tab
        } else {
            event.code
        };

        Self { code, modifiers }
    }
}

pub(super) fn parse_sequence(source: &str, leader: &str) -> Result<Vec<KeyChord>, String> {
    let mut sequence = Vec::new();
    let mut characters = source.char_indices().peekable();

    while let Some((start, character)) = characters.next() {
        if character != '<' {
            sequence.push(KeyChord {
                code: KeyCode::Char(character),
                modifiers: KeyModifiers::NONE,
            });
            continue;
        }

        let mut end = None;
        for (index, candidate) in characters.by_ref() {
            if candidate == '>' {
                end = Some(index);
                break;
            }
        }

        let end = end.ok_or_else(|| format!("unterminated key token in {source}"))?;
        let token = source[start + 1..end].to_ascii_lowercase();
        if token == "leader" {
            if leader.eq_ignore_ascii_case("<leader>") {
                return Err("mapleader cannot reference itself".to_owned());
            }

            sequence.extend(parse_sequence(leader, " ")?);
        } else {
            sequence.push(parse_token(&token)?);
        }
    }

    if sequence.is_empty() {
        Err("key sequence cannot be empty".to_owned())
    } else {
        Ok(sequence)
    }
}

pub(super) fn display_chord(chord: &KeyChord) -> String {
    if let KeyCode::Char(character) = chord.code
        && character != ' '
        && chord.modifiers.is_empty()
    {
        return character.to_string();
    }

    let name = match chord.code {
        KeyCode::Char(' ') => "Space".to_owned(),
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Esc => "Esc".to_owned(),
        KeyCode::Enter => "Enter".to_owned(),
        KeyCode::Backspace => "BS".to_owned(),
        KeyCode::Tab => "Tab".to_owned(),
        KeyCode::Up => "Up".to_owned(),
        KeyCode::Down => "Down".to_owned(),
        KeyCode::Left => "Left".to_owned(),
        KeyCode::Right => "Right".to_owned(),
        KeyCode::Home => "Home".to_owned(),
        KeyCode::End => "End".to_owned(),
        KeyCode::Delete => "Del".to_owned(),
        KeyCode::F(number) => format!("F{number}"),
        _ => "?".to_owned(),
    };

    let mut prefix = String::new();
    if chord.modifiers.contains(KeyModifiers::CONTROL) {
        prefix.push_str("C-");
    }
    if chord.modifiers.contains(KeyModifiers::SHIFT) {
        prefix.push_str("S-");
    }
    if chord.modifiers.contains(KeyModifiers::ALT) {
        prefix.push_str("A-");
    }

    format!("<{prefix}{name}>")
}

fn parse_token(token: &str) -> Result<KeyChord, String> {
    let mut modifiers = KeyModifiers::NONE;
    let mut name = token;
    loop {
        if let Some(value) = name.strip_prefix("c-") {
            modifiers.insert(KeyModifiers::CONTROL);
            name = value;
        } else if let Some(value) = name.strip_prefix("s-") {
            modifiers.insert(KeyModifiers::SHIFT);
            name = value;
        } else if let Some(value) = name
            .strip_prefix("a-")
            .or_else(|| name.strip_prefix("alt-"))
        {
            modifiers.insert(KeyModifiers::ALT);
            name = value;
        } else {
            break;
        }
    }

    let code = match name {
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "esc" => KeyCode::Esc,
        "enter" | "cr" => KeyCode::Enter,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        value if function_key(value).is_some() => {
            KeyCode::F(function_key(value).expect("validated function key number"))
        }
        value if value.chars().count() == 1 => KeyCode::Char(value.chars().next().unwrap()),
        _ => return Err(format!("unknown key token: <{token}>")),
    };

    Ok(KeyChord { code, modifiers })
}

fn function_key(name: &str) -> Option<u8> {
    name.strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .filter(|number| (1..=24).contains(number))
}
