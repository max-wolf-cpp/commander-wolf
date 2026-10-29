const MAX_EXPANSIONS: usize = 1024;

pub(super) fn expand_masks(expression: &str) -> Result<Vec<String>, String> {
    let expressions = split_masks(expression)?;
    if expressions.iter().all(|mask| mask.trim().is_empty()) {
        return Err("finder pattern cannot be empty".into());
    }

    let mut expanded = Vec::new();
    for expression in expressions {
        if expression.trim().is_empty() {
            continue;
        }

        expand_braces(&expression, &mut expanded)?;
        if expanded.len() > MAX_EXPANSIONS {
            return Err(format!(
                "finder pattern exceeds the {MAX_EXPANSIONS} brace-expansion limit"
            ));
        }
    }

    Ok(expanded)
}

fn split_masks(expression: &str) -> Result<Vec<String>, String> {
    let mut masks = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0usize;
    let mut in_class = false;
    for character in expression.chars() {
        match character {
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '{' if !in_class => brace_depth += 1,
            '}' if !in_class => {
                brace_depth = brace_depth
                    .checked_sub(1)
                    .ok_or("unexpected closing brace in finder pattern")?;
            }
            ';' if !in_class && brace_depth == 0 => {
                masks.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }

    if in_class {
        return Err("unclosed character class in finder pattern".into());
    }

    if brace_depth != 0 {
        return Err("unclosed brace in finder pattern".into());
    }

    masks.push(current);

    Ok(masks)
}

fn expand_braces(pattern: &str, output: &mut Vec<String>) -> Result<(), String> {
    let Some((open, close)) = find_brace_pair(pattern)? else {
        output.push(pattern.to_owned());
        return Ok(());
    };

    let body = &pattern[open + 1..close];
    let alternatives = split_alternatives(body)?;
    if alternatives.len() < 2 {
        return Err("brace expression must contain at least two alternatives".into());
    }

    let suffix = &pattern[close + 1..];
    for alternative in alternatives {
        let candidate = format!("{}{}{}", &pattern[..open], alternative, suffix);
        expand_braces(&candidate, output)?;
        if output.len() > MAX_EXPANSIONS {
            return Err(format!(
                "finder pattern exceeds the {MAX_EXPANSIONS} brace-expansion limit"
            ));
        }
    }

    Ok(())
}

fn find_brace_pair(pattern: &str) -> Result<Option<(usize, usize)>, String> {
    let characters = pattern.char_indices().collect::<Vec<_>>();
    let mut class_depth = 0usize;
    let mut open = None;
    for (index, character) in &characters {
        match character {
            '[' => class_depth += 1,
            ']' => class_depth = class_depth.saturating_sub(1),
            '{' if class_depth == 0 => {
                open = Some(*index);
                break;
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return Ok(None);
    };

    let mut depth = 0usize;
    let mut in_class = false;
    let mut close = None;
    for (index, character) in pattern[open..].char_indices() {
        match character {
            '[' => in_class = true,
            ']' => in_class = false,
            '{' if !in_class => depth += 1,
            '}' if !in_class => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + index);
                    break;
                }
            }
            _ => {}
        }
    }

    let close = close.ok_or("unclosed brace in finder pattern")?;

    Ok(Some((open, close)))
}

fn split_alternatives(body: &str) -> Result<Vec<&str>, String> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_class = false;
    for (index, character) in body.char_indices() {
        match character {
            '[' => in_class = true,
            ']' => in_class = false,
            '{' if !in_class => depth += 1,
            '}' if !in_class => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("unexpected closing brace in finder pattern")?;
            }
            ',' if !in_class && depth == 0 => {
                result.push(&body[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }

    result.push(&body[start..]);
    if result.iter().any(|value| value.is_empty()) {
        return Err("brace alternatives cannot be empty".into());
    }

    Ok(result)
}
