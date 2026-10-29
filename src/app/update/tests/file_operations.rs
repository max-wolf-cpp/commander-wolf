use super::*;

#[test]
fn delete_requires_explicit_confirmation_and_keeps_the_saved_source() {
    let mut state = state();
    assert!(update(&mut state, Action::RequestDelete).is_empty());
    assert_eq!(state.mode, AppMode::Confirm);
    assert_eq!(state.dialog_kind, DialogKind::Warning);
    assert!(matches!(
        state.confirmation.as_ref().map(|value| value.operations.as_slice()),
        Some([FileOperation::Delete { source }]) if source == &PathBuf::from("a.png")
    ));

    state.workspace.active_mut().selected = 1;
    let effects = update(&mut state, Action::AcceptConfirmation);
    assert_eq!(state.mode, AppMode::Normal);
    assert!(matches!(
        effects.last(),
        Some(Effect::PerformFileOperationBatch(FileOperationBatchRequest {
            operations,
            ..
        })) if matches!(operations.as_slice(), [FileOperation::Delete { source }] if source == &PathBuf::from("a.png"))
    ));

    update(&mut state, Action::RequestDelete);
    assert!(update(&mut state, Action::CancelConfirmation).is_empty());
    assert_eq!(state.mode, AppMode::Normal);
    assert!(state.confirmation.is_none());
}

#[test]
fn parent_entry_is_rejected_for_every_selected_file_operation() {
    let mut state = state();
    state.workspace.active_mut().entries[0].is_parent = true;

    for action in [
        Action::CopyToOther,
        Action::MoveToOther,
        Action::RequestDelete,
        Action::Rename("new".into()),
    ] {
        assert!(update(&mut state, action).is_empty());
        assert!(
            state
                .notification
                .as_ref()
                .is_some_and(|notification| notification.message.contains("'..'"))
        );
    }
}

#[test]
fn copy_builds_resolved_paths_and_selection_policy() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = PathBuf::from("left");
    state.workspace.panel_mut(PanelId::Left).entries[0].path = PathBuf::from("left/a.png");
    state.workspace.panel_mut(PanelId::Right).directory = PathBuf::from("right");

    let effects = update(&mut state, Action::CopyToOther);
    assert!(matches!(
        effects.as_slice(),
        [Effect::PerformFileOperationBatch(FileOperationBatchRequest {
            operations,
            refresh,
            ..
        })] if matches!(operations.as_slice(), [FileOperation::Copy { source, destination, .. }]
                if source == &PathBuf::from("left/a.png")
                    && destination == &PathBuf::from("right/a.png"))
            && refresh[0].selection.preferred.as_ref() == Some(&PathBuf::from("left/a.png"))
    ));
}

#[test]
fn copy_and_move_conflicts_wait_for_a_choice_and_apply_overwrite_to_the_batch() {
    let root = std::env::temp_dir().join(format!(
        "commander-wolf-copy-dialog-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let left = root.join("left");
    let right = root.join("right");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::create_dir_all(&right).unwrap();
    for name in ["a.txt", "b.txt"] {
        std::fs::write(left.join(name), "new").unwrap();
        std::fs::write(right.join(name), "old").unwrap();
    }

    let mut state = state();
    state.workspace.panel_mut(PanelId::Left).directory = left.clone();
    state.workspace.panel_mut(PanelId::Right).directory = right.clone();
    state.workspace.panel_mut(PanelId::Left).entries = ["a.txt", "b.txt"]
        .into_iter()
        .map(|name| DirectoryEntry {
            path: left.join(name),
            name: name.into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        })
        .collect();

    update(&mut state, Action::ToggleMark);
    state.workspace.active_mut().selected = 1;
    update(&mut state, Action::ToggleMark);

    let effects = update(&mut state, Action::CopyToOther);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::PerformFileOperationBatch(_)))
    );
    assert_eq!(state.mode, AppMode::Confirm);
    assert_eq!(state.dialog_kind, DialogKind::Warning);
    assert_eq!(state.file_conflict.as_ref().unwrap().conflicts.len(), 2);

    let mut cancelled = state.clone();
    update(&mut cancelled, Action::CancelConfirmation);
    assert!(cancelled.file_conflict.is_none());
    assert_eq!(cancelled.workspace.active().marked.len(), 2);

    let mut skipped = state.clone();
    update(&mut skipped, Action::ToggleFileConflictApplyToAll);
    let effects = update(&mut skipped, Action::SkipFileConflict);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::PerformFileOperationBatch(_)))
    );
    assert!(skipped.workspace.active().marked.is_empty());
    assert_eq!(skipped.mode, AppMode::Normal);

    update(&mut state, Action::ToggleFileConflictApplyToAll);
    let effects = update(&mut state, Action::OverwriteFileConflict);
    assert!(matches!(
        effects.as_slice(),
        [Effect::PerformFileOperationBatch(request)]
            if request.operations.len() == 2
                && request.operations.iter().all(FileOperation::overwrites)
    ));
    assert!(state.file_conflict.is_none());
    assert_eq!(state.mode, AppMode::Normal);

    let effects = update(&mut state, Action::MoveToOther);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::PerformFileOperationBatch(_)))
    );
    assert_eq!(
        state
            .file_conflict
            .as_ref()
            .and_then(|conflict| conflict.request.kind()),
        Some(FileOperationKind::Move)
    );

    update(&mut state, Action::ToggleFileConflictApplyToAll);
    let effects = update(&mut state, Action::OverwriteFileConflict);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PerformFileOperationBatch(request)
            if request.operations.len() == 2
                && request.operations.iter().all(|operation| matches!(
                    operation,
                    FileOperation::Move { overwrite: true, .. }
                ))
    )));
    assert!(state.file_conflict.is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_operation_refreshes_both_panels_and_partial_is_visible() {
    let mut state = state();
    let Effect::PerformFileOperationBatch(request) =
        update(&mut state, Action::CopyToOther).pop().unwrap()
    else {
        panic!("operation expected")
    };

    let effects = update(
        &mut state,
        Action::FileOperationBatchCompleted {
            request,
            outcome: FileOperationOutcome::changed_failure("data copied, source remained"),
        },
    );
    assert!(matches!(effects.as_slice(), [Effect::RefreshPanels { .. }]));
    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.level == crate::app::NotificationLevel::Error
            && notification.message.contains("source remained")
    }));
}

#[test]
fn completed_operation_remains_successful_when_refresh_fails_and_preview_resynchronizes() {
    let mut state = state();
    state.workspace.preview_visible = true;
    let requests = refresh_requests(&state, None, None);
    let left_snapshot = DirectorySnapshot {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            path: PathBuf::from("renamed.png"),
            name: "renamed.png".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }],
        selected: 0,
    };
    let outcome = FileOperationOutcome::success("Rename completed");
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

    let effects = update(
        &mut state,
        Action::PanelsRefreshed {
            results,
            context: RefreshContext::Operation {
                kind: FileOperationKind::Rename,
                outcome,
            },
        },
    );

    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.message.contains("Rename completed")
            && notification.message.contains("filesystem changed")
            && notification.message.contains("right panel")
    }));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("renamed.png")
    )));
}

#[test]
fn destructive_operation_cleans_preview_before_filesystem_effect() {
    let mut state = state();
    state.workspace.preview_visible = true;
    state.preview.playback = PlaybackState::Playing;

    let effects = update(&mut state, Action::MoveToOther);
    assert!(matches!(effects.first(), Some(Effect::StopPreviewPlayback)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ClearMediaSurface))
    );
    assert!(matches!(
        effects.last(),
        Some(Effect::PerformFileOperationBatch(_))
    ));
}
