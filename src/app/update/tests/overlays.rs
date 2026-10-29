use super::*;

#[test]
fn copying_with_progress_restores_the_same_selected_image_after_refresh() {
    use crate::file_operations::progress::OperationProgress;
    use crate::preview::MediaMetadata;

    for batch in [false, true] {
        for outcome in [
            FileOperationOutcome::success("Copied"),
            FileOperationOutcome::changed_failure("Partially copied"),
            FileOperationOutcome::failure("Cancelled"),
            FileOperationOutcome::failure("Permission denied"),
        ] {
            for refresh_fails in [false, true] {
                let mut state = state();
                state.workspace.preview_visible = true;
                let path = state.selected_path().unwrap().to_path_buf();
                let generation = state.preview.begin(path.clone(), "a.png".into());
                let metadata = MediaMetadata {
                    kind: PreviewKind::Image,
                    pixel_width: 800,
                    pixel_height: 600,
                    duration: None,
                    has_audio: None,
                };
                state.preview.media = LoadState::Ready(metadata.clone());
                let entries = state.workspace.active().entries.clone();
                let refresh = refresh_requests(&state, None, None);
                let operation = FileOperation::Copy {
                    source: path.clone(),
                    destination: "copied.png".into(),
                    overwrite: false,
                };

                update(
                    &mut state,
                    Action::FileOperationProgressStarted(OperationProgress::new(
                        FileOperationKind::Copy,
                    )),
                );
                assert!(!state.preview.media_owns_surface());
                assert_eq!(state.preview.media, LoadState::Ready(metadata.clone()));

                let completed = if batch {
                    Action::FileOperationBatchCompleted {
                        request: FileOperationBatchRequest {
                            operations: vec![operation],
                            source_panel: PanelId::Left,
                            protected_directory: ".".into(),
                            refresh,
                        },
                        outcome: outcome.clone(),
                    }
                } else {
                    Action::FileOperationCompleted {
                        request: FileOperationRequest {
                            operation,
                            protected_directory: ".".into(),
                            refresh,
                        },
                        outcome: outcome.clone(),
                    }
                };
                let mut effects = update(&mut state, completed);
                if let [Effect::RefreshPanels { requests, context }] = effects.as_slice() {
                    let results = requests.clone().map(|request| PanelRefreshResult {
                        result: if refresh_fails {
                            Err("refresh failed".into())
                        } else {
                            Ok(DirectorySnapshot {
                                directory: request.directory.clone(),
                                entries: entries.clone(),
                                selected: 0,
                            })
                        },
                        request,
                    });
                    effects = update(
                        &mut state,
                        Action::PanelsRefreshed {
                            results,
                            context: context.clone(),
                        },
                    );
                }
                assert_eq!(state.selected_path(), Some(path.as_path()));
                assert!(effects.iter().any(|effect| matches!(effect,
                    Effect::PreparePreviewMedia { path: prepared, .. } if *prepared == path
                )));

                let restored_generation = state.preview.generation;
                update(
                    &mut state,
                    Action::PreviewMediaPrepared {
                        generation: restored_generation,
                        path,
                        metadata,
                    },
                );
                assert!(state.preview.media_owns_surface());
                assert!(restored_generation >= generation);
            }
        }
    }
}

#[test]
fn help_float_opens_scrolls_and_restores_underlying_view() {
    let mut state = state();
    state.workspace.preview_visible = true;
    state.preview.path = Some("a.png".into());
    state.preview.media = LoadState::Loading;

    assert_eq!(update(&mut state, Action::OpenHelp), vec![Effect::LoadHelp]);

    let mut maps = crate::input::Keymaps::default();
    for key in ["j", "k", "y"] {
        maps.set_with_description(
            crate::input::InputContext::Normal,
            key,
            "selection.next",
            Some(format!("Action {key}")),
            " ",
        )
        .unwrap();
    }

    let help = crate::help::HelpState::from_keymaps(&maps);
    assert_eq!(
        update(&mut state, Action::HelpLoaded(help)),
        vec![Effect::ClearMediaSurface]
    );
    assert!(state.help.is_some());

    update(&mut state, Action::ScrollHelp(1));
    assert_eq!(state.help.as_ref().unwrap().scroll, 0);

    let effects = update(&mut state, Action::CloseHelp);
    assert!(matches!(
        effects.as_slice(),
        [Effect::PreparePreviewMedia { path, .. }] if path == std::path::Path::new("a.png")
    ));
    assert!(state.help.is_none());
}

#[test]
fn closing_rename_and_failed_progress_restores_media_and_shell_refreshes() {
    use crate::file_operations::progress::OperationProgress;

    for mode in [IconMode::Unicode, IconMode::None] {
        for overlay in 0..3 {
            let mut state = state();
            state.workspace.icon_mode = mode;
            state.workspace.preview_visible = true;
            state.preview.path = Some("a.png".into());
            state.preview.media = LoadState::Loading;

            let action = match overlay {
                0 => Action::RequestRename { force_float: true },
                1 => Action::FileOperationProgressStarted(OperationProgress::new(
                    FileOperationKind::Copy,
                )),
                _ => Action::RunShellCommand("fixture".into()),
            };
            let effects = update(&mut state, action);
            assert!(matches!(effects.first(), Some(Effect::ClearMediaSurface)));
            assert!(
                !effects
                    .iter()
                    .any(|effect| matches!(effect, Effect::PreparePreviewMedia { .. }))
            );

            let effects = match overlay {
                0 => update(
                    &mut state,
                    Action::RenameInput(crate::dialogs::RenameInput::Cancel),
                ),
                1 => {
                    let request = FileOperationRequest {
                        operation: FileOperation::Copy {
                            source: "a.png".into(),
                            destination: "b.png".into(),
                            overwrite: false,
                        },
                        protected_directory: ".".into(),
                        refresh: refresh_requests(&state, None, None),
                    };
                    update(
                        &mut state,
                        Action::FileOperationCompleted {
                            request,
                            outcome: FileOperationOutcome::failure("Cancelled"),
                        },
                    )
                }
                _ => {
                    update(
                        &mut state,
                        Action::ShellFinished {
                            message: "launch failed".into(),
                            failed: true,
                        },
                    );
                    update(&mut state, Action::CloseShellOutput)
                }
            };

            if overlay == 2 {
                assert!(
                    effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::RefreshPanels { .. }))
                );
            } else {
                assert!(
                    effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::PreparePreviewMedia { .. }))
                );
            }
        }
    }
}

#[test]
fn directory_edit_allows_progress_and_cancellation_while_blocking_edit_input() {
    use crate::file_operations::progress::OperationProgress;

    let mut state = state();
    state.directory_edit.active = true;

    update(
        &mut state,
        Action::FileOperationProgressStarted(OperationProgress::new(FileOperationKind::Delete)),
    );

    assert!(state.operation_progress.is_some());
    assert!(update(&mut state, Action::Quit).is_empty());
    assert_eq!(
        update(&mut state, Action::CancelFileOperation),
        [Effect::CancelFileOperation]
    );
    assert!(state.operation_progress.as_ref().unwrap().cancelling);
}

#[test]
fn operation_progress_blocks_other_actions_and_keeps_cancellation_visible() {
    use crate::file_operations::progress::OperationProgress;

    let mut state = state();
    update(
        &mut state,
        Action::FileOperationProgressStarted(OperationProgress::new(FileOperationKind::Copy)),
    );

    let selected = state.workspace.active().selected;
    for action in [
        Action::Quit,
        Action::EnterCommandMode,
        Action::CopyToOther,
        Action::ToggleActivePanel,
        Action::MoveSelection { delta: 1, count: 1 },
        Action::RunShellCommand("ignored".into()),
    ] {
        assert!(update(&mut state, action).is_empty());
    }

    assert!(!state.should_quit);
    assert_eq!(state.mode, AppMode::Normal);
    assert_eq!(state.workspace.active().selected, selected);

    assert_eq!(
        update(&mut state, Action::CancelFileOperation),
        [Effect::CancelFileOperation]
    );

    update(
        &mut state,
        Action::FileOperationProgressUpdated(OperationProgress::new(FileOperationKind::Copy)),
    );
    assert!(state.operation_progress.as_ref().unwrap().cancelling);
    assert!(update(&mut state, Action::CancelFileOperation).is_empty());

    let request = FileOperationRequest {
        operation: FileOperation::Copy {
            source: "a".into(),
            destination: "b".into(),
            overwrite: false,
        },
        protected_directory: ".".into(),
        refresh: refresh_requests(&state, None, None),
    };
    update(
        &mut state,
        Action::FileOperationCompleted {
            request,
            outcome: crate::file_operations::FileOperationOutcome::failure("Cancelled"),
        },
    );
    assert!(state.operation_progress.is_none());

    update(&mut state, Action::EnterCommandMode);
    assert_eq!(state.mode, AppMode::Command);
}

#[test]
fn shell_output_blocks_dismissal_while_running_and_refreshes_afterwards() {
    let mut state = state();
    assert!(update(&mut state, Action::RunShellCommand(" ".into())).is_empty());
    assert!(state.shell_output.is_none());

    state.workspace.active_mut().directory = "test root".into();
    let effects = update(&mut state, Action::RunShellCommand("echo hello".into()));
    assert!(effects.contains(&Effect::RunShellCommand {
        command: "echo hello".into(),
        directory: "test root".into()
    }));
    assert!(effects.contains(&Effect::ClearMediaSurface));

    assert!(update(&mut state, Action::CloseShellOutput).is_empty());
    assert_eq!(
        update(&mut state, Action::CancelShellCommand),
        [Effect::CancelShellCommand]
    );

    update(&mut state, Action::ShellOutput("hello\nworld".into()));
    update(&mut state, Action::ScrollShellOutput(isize::MAX));
    assert_eq!(state.shell_output.as_ref().unwrap().scroll, 1);
    update(&mut state, Action::ScrollShellOutput(isize::MIN));
    assert_eq!(state.shell_output.as_ref().unwrap().scroll, 0);

    update(
        &mut state,
        Action::ShellFinished {
            message: "Exit code: 0".into(),
            failed: false,
        },
    );

    assert!(matches!(
        update(&mut state, Action::CloseShellOutput).as_slice(),
        [Effect::RefreshPanels { .. }]
    ));
    assert!(state.shell_output.is_none());
}
