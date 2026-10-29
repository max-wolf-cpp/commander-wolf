use crate::workspace::DirectoryEntry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

pub fn repeat_match(
    entries: &[DirectoryEntry],
    query: &str,
    selected: usize,
    direction: SearchDirection,
    count: usize,
) -> Option<usize> {
    find_match(entries, query, selected, direction, count.max(1), true)
}

pub fn match_ranges(value: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }

    value
        .match_indices(query)
        .map(|(start, value)| (start, start + value.len()))
        .collect()
}

pub(super) fn find_match(
    entries: &[DirectoryEntry],
    query: &str,
    selected: usize,
    direction: SearchDirection,
    count: usize,
    advance_first: bool,
) -> Option<usize> {
    if entries.is_empty() || query.is_empty() {
        return None;
    }

    let matches = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.name.contains(query).then_some(index))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return None;
    }

    let mut cursor = selected.min(entries.len() - 1);
    for step in 0..count {
        let advance = advance_first || step > 0 || !matches.contains(&cursor);
        if advance {
            cursor = match direction {
                SearchDirection::Forward => matches
                    .iter()
                    .copied()
                    .find(|index| *index > cursor)
                    .unwrap_or(matches[0]),
                SearchDirection::Backward => matches
                    .iter()
                    .copied()
                    .rev()
                    .find(|index| *index < cursor)
                    .unwrap_or(*matches.last().expect("matches are not empty")),
            };
        }
    }

    Some(cursor)
}
