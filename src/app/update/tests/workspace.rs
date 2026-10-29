use super::*;

#[test]
fn hidden_toggle_updates_both_panels_and_keeps_visible_targets() {
    let mut state = state();
    state.preview.playback = PlaybackState::Playing;
    for panel in &mut state.workspace.panels {
        panel.entries[0].is_hidden = true;
        panel.selected = 1;
    }

    let effects = update(&mut state, Action::ToggleHidden);
    assert!(matches!(effects.first(), Some(Effect::StopPreviewPlayback)));
    assert!(!state.workspace.show_hidden);
    for panel in &state.workspace.panels {
        assert_eq!(panel.entries.len(), 1);
        assert_eq!(panel.selected_path(), Some(std::path::Path::new("b.png")));
    }

    update(&mut state, Action::ToggleHidden);
    assert!(state.workspace.show_hidden);
    assert!(
        state
            .workspace
            .panels
            .iter()
            .all(|panel| panel.entries.len() == 2)
    );
}

#[test]
fn initializes_both_panels() {
    let mut state = state();
    let effects = update(&mut state, Action::Initialize);
    assert!(matches!(
        effects.as_slice(),
        [
            Effect::LoadDirectory {
                panel: PanelId::Left,
                ..
            },
            Effect::LoadDirectory {
                panel: PanelId::Right,
                ..
            }
        ]
    ));
}

#[test]
fn toggling_panel_preserves_independent_selection() {
    let mut state = state();
    update(&mut state, Action::MoveSelection { delta: 1, count: 1 });
    update(&mut state, Action::ToggleActivePanel);
    assert_eq!(state.workspace.active_panel, PanelId::Right);
    assert_eq!(state.workspace.panel(PanelId::Left).selected, 1);
    assert_eq!(state.workspace.panel(PanelId::Right).selected, 0);
}

#[test]
fn directory_navigation_targets_only_the_active_panel() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = PathBuf::from("root/child");
    state.workspace.panel_mut(PanelId::Left).entries[0] = DirectoryEntry {
        path: PathBuf::from("root/child/sub"),
        name: "sub".into(),
        is_directory: true,
        is_parent: false,
        is_hidden: false,
    };

    let right_before = state.workspace.panel(PanelId::Right).clone();
    let effects = update(&mut state, Action::OpenSelected);
    assert!(matches!(
        effects.last(),
        Some(Effect::LoadDirectory {
            panel: PanelId::Left,
            path,
            ..
        }) if path == &PathBuf::from("root/child/sub")
    ));
    assert_eq!(state.workspace.panel(PanelId::Right), &right_before);

    state.workspace.panel_mut(PanelId::Left).entries[0] = DirectoryEntry {
        path: PathBuf::from("root/child/wolf.png"),
        name: "wolf.png".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    assert!(update(&mut state, Action::OpenSelected).is_empty());
    assert!(update(&mut state, Action::OpenSelectedFile)
        .iter()
        .any(|effect| matches!(effect, Effect::ProbeViewer { path, .. } if path == &PathBuf::from("root/child/wolf.png"))));
    update(&mut state, Action::ViewerClose);

    let effects = update(&mut state, Action::OpenParent);
    assert!(matches!(
        effects.last(),
        Some(Effect::LoadDirectory {
            panel: PanelId::Left,
            path,
            preferred: Some(preferred),
        }) if path == &PathBuf::from("root") && preferred == &PathBuf::from("root/child")
    ));

    state.workspace.panel_mut(PanelId::Left).entries[0] = DirectoryEntry {
        path: PathBuf::from("root"),
        name: "..".into(),
        is_directory: true,
        is_parent: true,
        is_hidden: false,
    };
    let effects = update(&mut state, Action::OpenSelected);
    assert!(matches!(
        effects.last(),
        Some(Effect::LoadDirectory {
            path,
            preferred: Some(preferred),
            ..
        }) if path == &PathBuf::from("root") && preferred == &PathBuf::from("root/child")
    ));
}

#[test]
fn failed_load_preserves_panel_and_reports_the_error() {
    let mut state = state();
    let before = state.workspace.panel(PanelId::Left).clone();
    update(
        &mut state,
        Action::DirectoryLoadFailed {
            panel: PanelId::Left,
            message: "permission denied".into(),
        },
    );
    assert_eq!(state.workspace.panel(PanelId::Left), &before);
    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.level == crate::app::NotificationLevel::Error
            && notification.message == "permission denied"
    }));
}

#[test]
fn switching_panels_moves_and_synchronizes_open_preview() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    let effects = update(&mut state, Action::ToggleActivePanel);
    assert_eq!(state.workspace.preview_panel(), Some(PanelId::Left));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ClearMediaSurface))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia {
            panel: PanelId::Left,
            path,
            ..
        } if path == &PathBuf::from("a.png")
    )));
}

#[test]
fn change_directory_resolves_relative_and_absolute_paths_from_active_panel() {
    let mut state = state();
    let base = std::env::current_dir().unwrap();
    state.workspace.active_mut().directory = base.join("panel");
    let relative = update(
        &mut state,
        Action::ChangeDirectory("../目录 with spaces".into()),
    );
    assert!(matches!(
        relative.as_slice(),
        [Effect::LoadDirectory {
            path,
            panel: PanelId::Left,
            ..
        }] if path == &base.join("目录 with spaces")
    ));
    let absolute_target = base.join("absolute target");
    let absolute = update(
        &mut state,
        Action::ChangeDirectory(absolute_target.to_string_lossy().into_owned()),
    );
    assert!(matches!(
        absolute.as_slice(),
        [Effect::LoadDirectory { path, .. }] if path == &absolute_target
    ));
}
