use super::*;

#[test]
fn metadata_paging_reaches_wrapped_path_rows_and_reclamps_after_resize() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    let path = "C:/long directory/".repeat(40) + "end.txt";
    let generation = state.preview.generation;
    update(
        &mut state,
        Action::PreviewMetadataLoaded {
            generation,
            path: "a.png".into(),
            metadata: crate::preview::FileMetadata {
                path: path.clone(),
                file_type: "Regular file".into(),
                size: "1 B".into(),
                created: "unknown".into(),
                modified: "unknown".into(),
                permissions: "Read/write".into(),
                extension: Some("png".into()),
                hard_links: Some(1),
            },
        },
    );
    let layout = crate::ui::resolve_layout_with_styles(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
        state.status_line_options.style,
        state.workspace.panel_style,
    );
    let value_width = usize::from(layout.preview.unwrap().metadata.width - 2 - 13);
    let rows = state.preview.metadata_rows().len() - 1 + path.len().div_ceil(value_width);
    let max_offset = rows - state.preview.metadata_viewport.visible_rows;
    state.preview.playback = PlaybackState::Playing;
    for _ in 0..rows {
        assert!(update(&mut state, Action::PreviewMetadataPageDown).is_empty());
        assert_eq!(state.preview.playback, PlaybackState::Playing);
    }
    assert_eq!(state.preview.metadata_viewport.offset, max_offset);
    assert!(max_offset > state.preview.metadata_rows().len());
    assert!(update(&mut state, Action::PreviewMetadataPageUp).is_empty());
    assert!(state.preview.metadata_viewport.offset < max_offset);

    update(
        &mut state,
        Action::Resize {
            size: TerminalSize {
                columns: 4000,
                rows: 100,
            },
            panel_rows: 90,
            finder_rows: 80,
            tree_rows: 80,
        },
    );
    assert_eq!(state.preview.metadata_viewport.offset, 0);
}

#[test]
fn preview_results_are_generation_and_path_scoped() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);

    let generation = state.preview.generation;
    let metadata = crate::preview::FileMetadata {
        path: "a.png".into(),
        file_type: "Regular file".into(),
        size: "1 B (1 bytes)".into(),
        created: "Unavailable".into(),
        modified: "Unavailable".into(),
        permissions: "Read/write".into(),
        extension: Some("png".into()),
        hard_links: Some(1),
    };
    update(
        &mut state,
        Action::PreviewMetadataLoaded {
            generation: generation.saturating_sub(1),
            path: PathBuf::from("a.png"),
            metadata: metadata.clone(),
        },
    );
    assert!(matches!(state.preview.metadata, LoadState::Loading));

    update(
        &mut state,
        Action::PreviewMetadataLoaded {
            generation,
            path: PathBuf::from("a.png"),
            metadata: metadata.clone(),
        },
    );
    assert_eq!(state.preview.metadata, LoadState::Ready(metadata));

    update(
        &mut state,
        Action::PreviewMediaUnavailable {
            generation,
            path: PathBuf::from("other.png"),
            message: "stale".into(),
        },
    );
    assert!(matches!(state.preview.media, LoadState::Loading));

    let effects = update(
        &mut state,
        Action::PreviewMediaUnavailable {
            generation,
            path: PathBuf::from("a.png"),
            message: "unsupported media".into(),
        },
    );
    assert!(matches!(state.preview.text, LoadState::Loading));
    assert!(matches!(
        effects.as_slice(),
        [Effect::LoadPreviewText { path, .. }] if path == &PathBuf::from("a.png")
    ));

    update(
        &mut state,
        Action::PreviewTextLoaded {
            generation,
            path: PathBuf::from("a.png"),
            preview: crate::preview::TextPreview {
                highlighting: Ok(Vec::new()),
                lines: vec!["fn main() {}".into()],
                truncated: false,
            },
        },
    );
    assert_eq!(state.preview.kind, PreviewKind::Text);
    assert!(matches!(state.preview.text, LoadState::Ready(_)));
}

#[test]
fn preview_replaces_inactive_panel_without_destroying_it() {
    let mut state = state();
    state.workspace.panel_mut(PanelId::Right).selected = 1;

    let effects = update(&mut state, Action::TogglePreview);
    assert!(state.workspace.preview_visible);
    assert_eq!(state.workspace.preview_panel(), Some(PanelId::Right));
    assert_eq!(state.workspace.panel(PanelId::Right).selected, 1);
    assert!(matches!(effects.first(), Some(Effect::ClearMediaSurface)));
}

#[test]
fn directory_selection_loads_a_generation_scoped_content_preview() {
    let mut state = state();
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("folder"),
        name: "folder".into(),
        is_directory: true,
        is_parent: false,
        is_hidden: false,
    };

    let effects = update(&mut state, Action::TogglePreview);
    let generation = state.preview.generation;
    assert!(matches!(state.preview.directory, LoadState::Loading));
    assert!(matches!(state.preview.media, LoadState::Idle));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::LoadPreviewDirectory { path, .. } if path == &PathBuf::from("folder")
    )));

    update(
        &mut state,
        Action::PreviewDirectoryLoaded {
            generation,
            path: PathBuf::from("folder"),
            preview: crate::preview::DirectoryPreview {
                entries: vec![crate::preview::DirectoryPreviewEntry {
                    is_hidden: false,
                    name: "child.txt".into(),
                    is_directory: false,
                }],
                truncated: false,
            },
        },
    );
    assert_eq!(state.preview.kind, PreviewKind::Directory);
    assert!(matches!(state.preview.directory, LoadState::Ready(_)));
}

#[test]
fn supported_archive_falls_back_from_media_to_an_index_preview() {
    let mut state = state();
    state.workspace.active_mut().entries[0] = DirectoryEntry {
        path: PathBuf::from("stats.zip"),
        name: "stats.zip".into(),
        is_directory: false,
        is_parent: false,
        is_hidden: false,
    };
    update(&mut state, Action::TogglePreview);
    let generation = state.preview.generation;

    let effects = update(
        &mut state,
        Action::PreviewMediaUnavailable {
            generation,
            path: PathBuf::from("stats.zip"),
            message: "not media".into(),
        },
    );
    assert!(matches!(state.preview.archive, LoadState::Loading));
    assert!(matches!(state.preview.text, LoadState::Idle));
    assert!(matches!(
        effects.as_slice(),
        [Effect::LoadPreviewArchive { path, .. }] if path == &PathBuf::from("stats.zip")
    ));

    update(
        &mut state,
        Action::PreviewArchiveLoaded {
            generation,
            path: PathBuf::from("stats.zip"),
            preview: crate::preview::ArchivePreview {
                entries: vec![crate::preview::ArchivePreviewEntry {
                    path: "report.json".into(),
                    is_directory: false,
                }],
                truncated: false,
            },
        },
    );
    assert_eq!(state.preview.kind, PreviewKind::Archive);
    assert!(matches!(state.preview.archive, LoadState::Ready(_)));
}
