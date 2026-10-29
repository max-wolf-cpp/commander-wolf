use super::tokens::Token;

pub(super) fn matches_tokens(tokens: &[Token], value: &str) -> bool {
    let value = value.chars().collect::<Vec<_>>();
    let columns = value.len() + 1;
    // Cache each token/character position pair to avoid repeated wildcard backtracking.
    let mut memo = vec![None; (tokens.len() + 1) * columns];

    matches_at(tokens, &value, 0, 0, columns, &mut memo)
}

fn matches_at(
    tokens: &[Token],
    value: &[char],
    token: usize,
    character: usize,
    columns: usize,
    memo: &mut [Option<bool>],
) -> bool {
    let memo_index = token * columns + character;
    if let Some(result) = memo[memo_index] {
        return result;
    }

    let result = match tokens.get(token) {
        None => character == value.len(),
        Some(Token::Literal(expected)) => {
            value.get(character) == Some(expected)
                && matches_at(tokens, value, token + 1, character + 1, columns, memo)
        }
        Some(Token::Any) => {
            value.get(character).is_some_and(|value| *value != '/')
                && matches_at(tokens, value, token + 1, character + 1, columns, memo)
        }
        Some(Token::Class(class)) => value.get(character).is_some_and(|candidate| {
            *candidate != '/'
                && class.matches(*candidate)
                && matches_at(tokens, value, token + 1, character + 1, columns, memo)
        }),
        Some(Token::Star) => {
            matches_at(tokens, value, token + 1, character, columns, memo)
                || value.get(character).is_some_and(|candidate| {
                    *candidate != '/'
                        && matches_at(tokens, value, token, character + 1, columns, memo)
                })
        }
        Some(Token::GlobStar) => {
            matches_at(tokens, value, token + 1, character, columns, memo)
                || character < value.len()
                    && matches_at(tokens, value, token, character + 1, columns, memo)
        }
        Some(Token::GlobStarSlash) => {
            matches_at(tokens, value, token + 1, character, columns, memo)
                || value[character..]
                    .iter()
                    .enumerate()
                    .filter(|(_, candidate)| **candidate == '/')
                    .any(|(offset, _)| {
                        matches_at(
                            tokens,
                            value,
                            token + 1,
                            character + offset + 1,
                            columns,
                            memo,
                        )
                    })
        }
    };

    memo[memo_index] = Some(result);

    result
}
