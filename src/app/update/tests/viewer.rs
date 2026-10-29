use super::*;

#[test]
fn files_open_through_the_modal_probe_and_fail_without_changing_workspace() {
    let mut state = state();
    let workspace = state.workspace.clone();

    let effects = update(&mut state, Action::OpenSelectedFile);
    assert!(state.viewer.is_open());
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::ProbeViewer { path, .. } if path == &PathBuf::from("a.png")
    )));

    let generation = state.viewer.generation;
    update(
        &mut state,
        Action::ViewerFailed {
            generation,
            path: PathBuf::from("a.png"),
            message: "unsupported image".into(),
        },
    );
    assert!(!state.viewer.is_open());
    assert_eq!(state.workspace, workspace);
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| notification.message.contains("unsupported image"))
    );
}

#[test]
fn enter_opens_webp_in_the_internal_viewer() {
    let mut state = state();
    state.workspace.active_mut().entries[0].path = PathBuf::from("wolf.webp");
    state.workspace.active_mut().entries[0].name = "wolf.webp".into();

    let effects = update(&mut state, Action::OpenSelectedFile);

    assert!(state.viewer.is_open());
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::ProbeViewer { path, .. } if path == &PathBuf::from("wolf.webp")
    )));
}

#[test]
fn viewer_close_and_resize_clear_the_old_surface_before_repreparing() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    update(&mut state, Action::OpenSelectedFile);

    let first_generation = state.viewer.generation;
    let effects = update(
        &mut state,
        Action::Resize {
            size: TerminalSize {
                columns: 120,
                rows: 40,
            },
            panel_rows: 35,
            finder_rows: 20,
            tree_rows: 20,
        },
    );
    assert!(matches!(effects.first(), Some(Effect::StopPreviewPlayback)));
    assert!(matches!(effects.get(1), Some(Effect::ClearMediaSurface)));
    assert!(state.viewer.generation > first_generation);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::ProbeViewer { generation, .. } if *generation == state.viewer.generation
    )));

    let effects = update(&mut state, Action::ViewerClose);
    assert!(!state.viewer.is_open());
    assert!(matches!(effects.first(), Some(Effect::ClearMediaSurface)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::PreparePreviewMedia { .. }))
    );
}
