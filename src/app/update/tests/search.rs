use super::*;

#[test]
fn incremental_search_cancels_to_origin_and_confirmed_repeats_wrap_with_counts() {
    let mut state = state();
    state.workspace.preview_visible = true;
    assert!(update(&mut state, Action::SearchBegin).is_empty());
    assert_eq!(state.mode, AppMode::Search);
    assert_eq!(state.search.panel, Some(PanelId::Left));
    assert_eq!(state.search.original_selection, 0);

    let effects = update(&mut state, Action::SearchPushCharacter('b'));
    assert_eq!(state.workspace.active().selected, 1);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("b.png")
    )));

    update(&mut state, Action::SearchCancel);
    assert_eq!(state.workspace.active().selected, 0);
    assert_eq!(state.mode, AppMode::Normal);

    update(&mut state, Action::SearchBegin);
    for character in "png".chars() {
        update(&mut state, Action::SearchPushCharacter(character));
    }
    update(&mut state, Action::SearchSubmit);
    assert_eq!(state.mode, AppMode::Search);
    assert!(!state.search.active);
    assert_eq!(state.search.last_confirmed_query.as_deref(), Some("png"));
    assert!(state.search.confirmed_active);
    assert_eq!(state.search.highlight_query.as_deref(), Some("png"));

    update(
        &mut state,
        Action::SearchRepeat {
            direction: SearchDirection::Forward,
            count: 2,
        },
    );
    assert_eq!(state.workspace.active().selected, 1);
    update(
        &mut state,
        Action::SearchRepeat {
            direction: SearchDirection::Backward,
            count: 1,
        },
    );
    assert_eq!(state.workspace.active().selected, 0);

    update(
        &mut state,
        Action::SearchRepeat {
            direction: SearchDirection::Forward,
            count: 1,
        },
    );
    assert_eq!(state.workspace.active().selected, 1);
    assert_eq!(state.mode, AppMode::Search);

    update(&mut state, Action::SearchCancel);
    assert_eq!(state.mode, AppMode::Normal);
    assert_eq!(state.workspace.active().selected, 1);
    assert!(!state.search.confirmed_active);
    assert!(state.search.highlight_query.is_none());
    update(
        &mut state,
        Action::SearchRepeat {
            direction: SearchDirection::Backward,
            count: 1,
        },
    );
    assert_eq!(state.workspace.active().selected, 1);
}

#[test]
fn search_backspace_is_unicode_safe_and_refresh_clears_only_visuals() {
    let mut state = state();
    update(&mut state, Action::SearchBegin);
    update(&mut state, Action::SearchPushCharacter('猫'));
    update(&mut state, Action::SearchBackspace);
    assert!(state.search.query.is_empty());

    update(&mut state, Action::SearchBackspace);
    assert_eq!(state.mode, AppMode::Normal);

    state.search.last_confirmed_query = Some("png".into());
    state.search.highlight_panel = Some(PanelId::Left);
    state.search.highlight_query = Some("png".into());

    let requests = refresh_requests(&state, None, None);
    let results = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshResult {
        request: requests[panel.index()].clone(),
        result: Ok(DirectorySnapshot {
            directory: PathBuf::from("."),
            entries: state.workspace.panel(panel).entries.clone(),
            selected: state.workspace.panel(panel).selected,
        }),
    });
    update(
        &mut state,
        Action::PanelsRefreshed {
            results,
            context: RefreshContext::Manual,
        },
    );
    assert!(state.search.highlight_query.is_none());
    assert_eq!(state.search.last_confirmed_query.as_deref(), Some("png"));
}
