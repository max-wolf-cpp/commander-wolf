#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Token {
    Literal(char),
    Any,
    Star,
    GlobStar,
    GlobStarSlash,
    Class(CharacterClass),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CharacterClass {
    negated: bool,
    ranges: Vec<(char, char)>,
}

pub(super) fn tokenize(pattern: &str) -> Result<Vec<Token>, String> {
    let chars = pattern.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '*' if chars.get(index + 1) == Some(&'*') => {
                if chars.get(index + 2) == Some(&'/') {
                    tokens.push(Token::GlobStarSlash);
                    index += 3;
                } else {
                    tokens.push(Token::GlobStar);
                    index += 2;
                }
            }
            '*' => {
                tokens.push(Token::Star);
                index += 1;
            }
            '?' => {
                tokens.push(Token::Any);
                index += 1;
            }
            '[' => {
                let (class, next) = parse_class(&chars, index)?;
                tokens.push(Token::Class(class));
                index = next;
            }
            character => {
                tokens.push(Token::Literal(character));
                index += 1;
            }
        }
    }

    Ok(tokens)
}

impl CharacterClass {
    pub(super) fn matches(&self, character: char) -> bool {
        let included = self
            .ranges
            .iter()
            .any(|(first, last)| (*first..=*last).contains(&character));

        included != self.negated
    }
}

fn parse_class(chars: &[char], start: usize) -> Result<(CharacterClass, usize), String> {
    let mut index = start + 1;
    let negated = matches!(chars.get(index), Some('!' | '^'));
    if negated {
        index += 1;
    }

    let mut ranges = Vec::new();
    while index < chars.len() && chars[index] != ']' {
        let first = chars[index];
        if chars.get(index + 1) == Some(&'-')
            && chars
                .get(index + 2)
                .is_some_and(|character| *character != ']')
        {
            let last = chars[index + 2];
            if first > last {
                return Err(format!("invalid character class range {first}-{last}"));
            }

            ranges.push((first, last));
            index += 3;
        } else {
            ranges.push((first, first));
            index += 1;
        }
    }

    if chars.get(index) != Some(&']') {
        return Err("unclosed character class in finder pattern".into());
    }

    if ranges.is_empty() {
        return Err("empty character class in finder pattern".into());
    }

    Ok((CharacterClass { negated, ranges }, index + 1))
}
