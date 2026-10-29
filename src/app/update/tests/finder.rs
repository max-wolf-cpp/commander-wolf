use super::*;

#[test]
fn finder_streams_current_generation_sorts_navigates_and_reveals() {
    let mut state = state();
    state.workspace.preview_visible = true;
    let right_before = state.workspace.panel(PanelId::Right).clone();

    let effects = update(&mut state, Action::FinderOpen);
    assert_eq!(state.finder.mode, FinderMode::Insert);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ClearMediaSurface))
    );

    for character in "*.png".chars() {
        update(&mut state, Action::FinderPushCharacter(character));
    }
    let effects = update(&mut state, Action::FinderSubmit);
    assert!(matches!(
        effects.as_slice(),
        [
            Effect::ClearMediaSurface,
            Effect::CancelDocumentPreview,
            Effect::StartFinderSearch(_)
        ]
    ));
    assert_eq!(state.finder.mode, FinderMode::Searching);

    let generation = state.finder.generation;
    let result = |path: &str| FinderResult {
        is_hidden: false,
        absolute_path: PathBuf::from(path),
        relative_path: path.into(),
        kind: FinderEntryKind::File,
    };
    update(
        &mut state,
        Action::FinderResultsAdded {
            generation: generation + 1,
            results: vec![result("stale.png")],
        },
    );
    assert!(state.finder.results.is_empty());

    let preview_effects = update(
        &mut state,
        Action::FinderResultsAdded {
            generation,
            results: vec![result("z.png"), result("a.png")],
        },
    );
    assert!(preview_effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("z.png")
    )));
    assert_eq!(state.preview.path, Some(PathBuf::from("z.png")));

    update(
        &mut state,
        Action::FinderCompleted {
            generation,
            skipped: 2,
            truncated: false,
            cancelled: false,
        },
    );
    assert_eq!(state.finder.mode, FinderMode::Normal);
    assert_eq!(state.finder.results[0].relative_path, "a.png");
    let preview_effects = update(
        &mut state,
        Action::FinderSelect {
            motion: SelectionMotion::First,
            count: None,
        },
    );
    assert!(preview_effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("a.png")
    )));
    update(
        &mut state,
        Action::FinderSelect {
            motion: SelectionMotion::Last,
            count: None,
        },
    );
    assert_eq!(state.finder.selected, 1);
    assert!(matches!(
        update(&mut state, Action::FinderOpenSelected).as_slice(),
        [Effect::RevealFinderResult { path, .. }] if path == &PathBuf::from("z.png")
    ));

    let effects = update(
        &mut state,
        Action::FinderRevealLoaded {
            generation,
            panel: PanelId::Left,
            directory: PathBuf::from("found"),
            entries: vec![DirectoryEntry {
                path: PathBuf::from("found/z.png"),
                name: "z.png".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            }],
            selected: 0,
        },
    );
    assert_eq!(state.finder.mode, FinderMode::Closed);
    assert_eq!(state.workspace.active().directory, PathBuf::from("found"));
    assert_eq!(state.workspace.panel(PanelId::Right), &right_before);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("found/z.png")
    )));
}

#[test]
fn finder_cancel_keeps_streamed_results_and_failed_reveal_stays_open() {
    let mut state = state();
    update(&mut state, Action::FinderOpen);
    state.finder.pattern = "*".into();
    update(&mut state, Action::FinderSubmit);

    let generation = state.finder.generation;
    update(
        &mut state,
        Action::FinderResultsAdded {
            generation,
            results: vec![FinderResult {
                is_hidden: false,
                absolute_path: PathBuf::from("entry"),
                relative_path: "entry".into(),
                kind: FinderEntryKind::Directory,
            }],
        },
    );

    assert_eq!(
        update(&mut state, Action::FinderCancel),
        vec![Effect::CancelFinderSearch]
    );
    assert_eq!(state.finder.mode, FinderMode::Normal);
    assert_eq!(state.finder.results.len(), 1);

    update(
        &mut state,
        Action::FinderRevealFailed {
            generation,
            message: "disappeared".into(),
        },
    );
    assert!(state.finder.is_open());
    assert_eq!(state.finder.error.as_deref(), Some("disappeared"));
}
