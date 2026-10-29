use super::*;

#[test]
fn refresh_updates_panels_independently_and_preserves_failed_state() {
    let mut state = state();
    let right_before = state.workspace.panel(PanelId::Right).clone();
    let requests = refresh_requests(&state, None, None);
    let left_snapshot = DirectorySnapshot {
        directory: PathBuf::from("new-left"),
        entries: vec![DirectoryEntry {
            path: PathBuf::from("new-left/new.txt"),
            name: "new.txt".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }],
        selected: 0,
    };
    let results = [
        PanelRefreshResult {
            request: requests[0].clone(),
            result: Ok(left_snapshot),
        },
        PanelRefreshResult {
            request: requests[1].clone(),
            result: Err("permission denied".into()),
        },
    ];

    update(
        &mut state,
        Action::PanelsRefreshed {
            results,
            context: RefreshContext::Manual,
        },
    );

    assert_eq!(
        state.workspace.panel(PanelId::Left).directory,
        PathBuf::from("new-left")
    );
    assert_eq!(state.workspace.panel(PanelId::Right), &right_before);
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| { notification.message.contains("right panel") })
    );
}

#[test]
fn watched_refresh_keeps_each_panels_selection_and_marks() {
    let mut state = state();
    let shared = PathBuf::from("shared");
    let entries = ["a.txt", "c.txt"].map(|name| DirectoryEntry {
        path: shared.join(name),
        name: name.into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    });
    let scroll_offset = state.workspace.scroll_offset;
    for (panel, selected) in [(PanelId::Left, 1), (PanelId::Right, 0)] {
        state.workspace.panel_mut(panel).replace_directory(
            shared.clone(),
            entries.to_vec(),
            selected,
            scroll_offset,
        );
    }
    state
        .workspace
        .panel_mut(PanelId::Left)
        .toggle_mark(&shared.join("c.txt"));

    let requests = refresh_requests(&state, None, None);
    let mut changed = entries.to_vec();
    changed.insert(
        1,
        DirectoryEntry {
            path: shared.join("b.txt"),
            name: "b.txt".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        },
    );
    let results = requests.map(|request| PanelRefreshResult {
        request,
        result: Ok(DirectorySnapshot {
            directory: shared.clone(),
            entries: changed.clone(),
            selected: 0,
        }),
    });

    let effects = update(
        &mut state,
        Action::WatchedPanelsRefreshed {
            results: Box::new(results),
            changed_paths: vec![shared.join("b.txt")],
        },
    );
    assert!(effects.is_empty());
    assert_eq!(
        state.workspace.panel(PanelId::Left).selected_path(),
        Some(shared.join("c.txt").as_path())
    );
    assert_eq!(
        state.workspace.panel(PanelId::Right).selected_path(),
        Some(shared.join("a.txt").as_path())
    );
    assert!(
        state
            .workspace
            .panel(PanelId::Left)
            .is_marked(&shared.join("c.txt"))
    );
    assert!(state.notification.is_none());
}

#[test]
fn watched_change_to_selected_file_reloads_preview_without_replacing_panel() {
    let mut state = state();
    let entries = state.workspace.active().entries.clone();
    let scroll_offset = state.workspace.scroll_offset;
    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).replace_directory(
            PathBuf::from("shared"),
            entries.clone(),
            0,
            scroll_offset,
        );
    }
    state.workspace.preview_visible = true;
    let requests = refresh_requests(&state, None, None);
    let results = requests.map(|request| PanelRefreshResult {
        request,
        result: Ok(DirectorySnapshot {
            directory: PathBuf::from("shared"),
            entries: entries.clone(),
            selected: 0,
        }),
    });

    let effects = update(
        &mut state,
        Action::WatchedPanelsRefreshed {
            results: Box::new(results),
            changed_paths: vec![PathBuf::from("a.png")],
        },
    );

    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::LoadPreviewMetadata { path, .. } if path == &PathBuf::from("a.png")
    )));
}

#[test]
fn refresh_keeps_independent_selections_when_both_panels_share_a_directory() {
    let mut state = state();
    let shared = PathBuf::from("shared");
    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).directory = shared.clone();
    }
    state.workspace.panel_mut(PanelId::Right).selected = 1;
    let requests = refresh_requests(&state, None, None);
    let entries = state.workspace.active().entries.clone();
    let results = [
        PanelRefreshResult {
            request: requests[0].clone(),
            result: Ok(DirectorySnapshot {
                directory: shared.clone(),
                entries: entries.clone(),
                selected: 0,
            }),
        },
        PanelRefreshResult {
            request: requests[1].clone(),
            result: Ok(DirectorySnapshot {
                directory: shared.clone(),
                entries,
                selected: 1,
            }),
        },
    ];

    update(
        &mut state,
        Action::PanelsRefreshed {
            results,
            context: RefreshContext::Manual,
        },
    );

    assert_eq!(state.workspace.panel(PanelId::Left).directory, shared);
    assert_eq!(state.workspace.panel(PanelId::Right).directory, shared);
    assert_eq!(state.workspace.panel(PanelId::Left).selected, 0);
    assert_eq!(state.workspace.panel(PanelId::Right).selected, 1);
}
