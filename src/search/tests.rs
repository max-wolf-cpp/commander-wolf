use std::path::PathBuf;

use super::*;

fn entries() -> Vec<DirectoryEntry> {
    ["alpha", "猫-alpha", "other", "alpha-end"]
        .into_iter()
        .map(|name| DirectoryEntry {
            path: PathBuf::from(name),
            name: name.into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        })
        .collect()
}

#[test]
fn draft_search_is_unicode_aware_and_no_match_restores_origin() {
    let mut search = SearchState::default();
    search.begin(PanelId::Left, 0);
    search.query.push('猫');
    assert_eq!(search.update_draft(&entries()), 1);
    search.query.pop();
    search.query.push('z');
    assert_eq!(search.update_draft(&entries()), 0);
    assert!(search.no_match);
}

#[test]
fn repeats_wrap_in_both_directions_and_honor_count() {
    let entries = entries();
    assert_eq!(
        repeat_match(&entries, "alpha", 0, SearchDirection::Forward, 2),
        Some(3)
    );
    assert_eq!(
        repeat_match(&entries, "alpha", 0, SearchDirection::Backward, 1),
        Some(3)
    );
}

#[test]
fn cancel_restores_previous_confirmed_highlight() {
    let mut search = SearchState {
        confirmed_active: true,
        last_confirmed_query: Some("old".into()),
        highlight_panel: Some(PanelId::Right),
        highlight_query: Some("old".into()),
        current_match: Some(4),
        ..SearchState::default()
    };
    search.begin(PanelId::Left, 2);
    search.query = "new".into();
    search.update_draft(&entries());
    assert_eq!(search.cancel(), 2);
    assert_eq!(search.highlighted_query(PanelId::Right), Some("old"));
    assert_eq!(search.last_confirmed_query.as_deref(), Some("old"));
    assert!(search.confirmed_active);
}

#[test]
fn dismissing_a_confirmed_search_keeps_history_but_clears_the_session() {
    let mut search = SearchState {
        confirmed_active: true,
        last_confirmed_query: Some("alpha".into()),
        highlight_panel: Some(PanelId::Left),
        highlight_query: Some("alpha".into()),
        current_match: Some(3),
        ..SearchState::default()
    };
    search.dismiss_confirmed();
    assert!(!search.confirmed_active);
    assert!(search.highlight_query.is_none());
    assert!(search.current_match.is_none());
    assert_eq!(search.last_confirmed_query.as_deref(), Some("alpha"));
}

#[test]
fn reports_every_non_overlapping_occurrence() {
    assert_eq!(match_ranges("foo-foo-猫", "foo"), vec![(0, 3), (4, 7)]);
}
