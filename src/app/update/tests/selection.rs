use super::*;

#[test]
fn repeated_motion_at_list_boundaries_preserves_preview_and_playback() {
    for row_count in [0_usize, 1, 2] {
        for (selected, delta, motion) in [
            (0, -1, SelectionMotion::Previous),
            (row_count.saturating_sub(1), 1, SelectionMotion::Next),
        ] {
            for kind in [PreviewKind::Image, PreviewKind::Video, PreviewKind::Audio] {
                let mut state = state();
                state.workspace.preview_visible = true;
                state.workspace.active_mut().entries.truncate(row_count);
                state.workspace.active_mut().selected = selected;
                state.preview.begin("a.png".into(), "a.png".into());
                state.preview.kind = kind;
                state.preview.media = LoadState::Ready(crate::preview::MediaMetadata {
                    kind,
                    pixel_width: 800,
                    pixel_height: 600,
                    duration: None,
                    has_audio: None,
                });
                if kind != PreviewKind::Image {
                    state.preview.playback = PlaybackState::Playing;
                }
                let before = state.clone();

                for count in [1, usize::MAX] {
                    for _ in 0..10 {
                        assert!(
                            update(&mut state, Action::MoveSelection { delta, count }).is_empty()
                        );
                        assert!(
                            update(
                                &mut state,
                                Action::Select {
                                    motion,
                                    count: Some(count)
                                }
                            )
                            .is_empty()
                        );
                        assert_eq!(state, before);
                    }
                }
            }
        }
    }
}

#[test]
fn moving_to_a_different_row_still_stops_playback_and_loads_the_new_preview() {
    let mut state = state();
    state.workspace.preview_visible = true;
    state.preview.kind = PreviewKind::Video;
    state.preview.playback = PlaybackState::Playing;
    let generation = state.preview.generation;

    let effects = update(&mut state, Action::MoveSelection { delta: 1, count: 1 });

    assert_eq!(state.workspace.active().selected, 1);
    assert!(effects.contains(&Effect::StopPreviewPlayback));
    assert!(effects.contains(&Effect::ClearMediaSurface));
    assert!(effects.iter().any(|effect| matches!(effect,
        Effect::PreparePreviewMedia { path, .. } if path == std::path::Path::new("b.png")
    )));
    assert_ne!(state.preview.generation, generation);
}

#[test]
fn marked_entries_drive_batch_operations_and_clear_only_after_completion() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = PathBuf::from("left");
    state.workspace.panel_mut(PanelId::Right).directory = PathBuf::from("right");
    for entry in &mut state.workspace.panel_mut(PanelId::Left).entries {
        entry.path = PathBuf::from("left").join(&entry.name);
    }

    update(&mut state, Action::ToggleMark);
    state.workspace.active_mut().selected = 1;
    update(&mut state, Action::ToggleMark);
    assert_eq!(state.workspace.active().marked.len(), 2);

    let Effect::PerformFileOperationBatch(request) =
        update(&mut state, Action::CopyToOther).pop().unwrap()
    else {
        panic!("batch operation expected")
    };

    assert_eq!(request.operations.len(), 2);
    assert!(request.operations.iter().any(|operation| matches!(
        operation,
        FileOperation::Copy { source, destination, .. }
            if source == &PathBuf::from("left/a.png")
                && destination == &PathBuf::from("right/a.png")
    )));
    assert_eq!(state.workspace.active().marked.len(), 2);

    let effects = update(
        &mut state,
        Action::FileOperationBatchCompleted {
            request,
            outcome: FileOperationOutcome::success("copied"),
        },
    );
    assert!(state.workspace.active().marked.is_empty());
    assert!(matches!(effects.as_slice(), [Effect::RefreshPanels { .. }]));
}

#[test]
fn existing_marks_replace_the_unmarked_current_entry_as_operation_targets() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = PathBuf::from("left");
    state.workspace.panel_mut(PanelId::Right).directory = PathBuf::from("right");
    for entry in &mut state.workspace.panel_mut(PanelId::Left).entries {
        entry.path = PathBuf::from("left").join(&entry.name);
    }
    update(&mut state, Action::ToggleMark);
    state.workspace.active_mut().selected = 1;

    let Effect::PerformFileOperationBatch(request) =
        update(&mut state, Action::MoveToOther).pop().unwrap()
    else {
        panic!("batch operation expected")
    };

    assert!(matches!(
        request.operations.as_slice(),
        [FileOperation::Move {
            source,
            destination,
            overwrite: false,
        }]
            if source == &PathBuf::from("left/a.png")
                && destination == &PathBuf::from("right/a.png")
    ));
}

#[test]
fn marked_delete_confirmation_keeps_marks_when_cancelled() {
    let mut state = state();
    update(&mut state, Action::ToggleMark);
    state.workspace.active_mut().selected = 1;
    update(&mut state, Action::ToggleMark);

    update(&mut state, Action::RequestDelete);
    assert_eq!(state.confirmation.as_ref().unwrap().operations.len(), 2);

    update(&mut state, Action::CancelConfirmation);
    assert_eq!(state.workspace.active().marked.len(), 2);
}

#[test]
fn visual_selection_expands_with_motion_and_escape_keeps_the_cursor() {
    let mut state = state();
    update(&mut state, Action::VisualBegin);
    assert_eq!(state.mode, AppMode::Visual);
    assert_eq!(state.visual_selection.panel, Some(PanelId::Left));
    assert_eq!(state.visual_selection.anchor, 0);

    update(&mut state, Action::MoveSelection { delta: 1, count: 1 });
    assert_eq!(state.workspace.active().selected, 1);

    update(&mut state, Action::VisualCancel);
    assert_eq!(state.mode, AppMode::Normal);
    assert_eq!(state.workspace.active().selected, 1);
    assert!(state.visual_selection.panel.is_none());
}

#[test]
fn visual_range_and_manual_marks_form_one_batch_then_leave_visual_mode() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = PathBuf::from("left");
    state.workspace.panel_mut(PanelId::Right).directory = PathBuf::from("right");
    state
        .workspace
        .panel_mut(PanelId::Left)
        .entries
        .push(DirectoryEntry {
            path: PathBuf::from("left/c.png"),
            name: "c.png".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        });
    for entry in &mut state.workspace.panel_mut(PanelId::Left).entries {
        entry.path = PathBuf::from("left").join(&entry.name);
    }
    state.workspace.active_mut().selected = 2;
    update(&mut state, Action::ToggleMark);
    state.workspace.active_mut().selected = 0;
    update(&mut state, Action::VisualBegin);
    update(&mut state, Action::MoveSelection { delta: 1, count: 1 });

    let Effect::PerformFileOperationBatch(request) =
        update(&mut state, Action::CopyToOther).pop().unwrap()
    else {
        panic!("batch operation expected")
    };

    assert_eq!(request.operations.len(), 3);
    assert_eq!(state.mode, AppMode::Normal);
    assert!(state.visual_selection.panel.is_none());
    assert_eq!(state.workspace.active().marked.len(), 1);
}

#[test]
fn cancelling_visual_delete_returns_to_visual_until_the_operation_is_accepted() {
    let mut state = state();
    update(&mut state, Action::VisualBegin);
    update(&mut state, Action::MoveSelection { delta: 1, count: 1 });
    update(&mut state, Action::RequestDelete);
    assert_eq!(state.mode, AppMode::Confirm);
    assert_eq!(state.confirmation.as_ref().unwrap().operations.len(), 2);

    update(&mut state, Action::CancelConfirmation);
    assert_eq!(state.mode, AppMode::Visual);
    assert!(state.visual_selection.panel.is_some());

    update(&mut state, Action::RequestDelete);
    let effects = update(&mut state, Action::AcceptConfirmation);
    assert_eq!(state.mode, AppMode::Normal);
    assert!(state.visual_selection.panel.is_none());
    assert!(matches!(
        effects.last(),
        Some(Effect::PerformFileOperationBatch(request)) if request.operations.len() == 2
    ));
}

#[test]
fn explicit_count_changes_last_motion_to_absolute() {
    let mut state = state();
    update(
        &mut state,
        Action::Select {
            motion: SelectionMotion::Last,
            count: None,
        },
    );
    assert_eq!(state.workspace.active().selected, 1);

    update(
        &mut state,
        Action::Select {
            motion: SelectionMotion::Last,
            count: Some(1),
        },
    );
    assert_eq!(state.workspace.active().selected, 0);
}
