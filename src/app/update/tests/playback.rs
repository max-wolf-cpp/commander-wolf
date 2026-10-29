use super::*;

#[test]
fn metadata_scrolling_never_stops_video_and_mute_can_be_armed_while_stopped() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    state.preview.metadata = LoadState::Ready(crate::preview::FileMetadata {
        path: "a.png".into(),
        file_type: "Regular file".into(),
        size: "1 B (1 bytes)".into(),
        created: "Unavailable".into(),
        modified: "Unavailable".into(),
        permissions: "Read/write".into(),
        extension: Some("png".into()),
        hard_links: Some(1),
    });
    state.preview.media = LoadState::Ready(crate::preview::MediaMetadata {
        kind: PreviewKind::Video,
        pixel_width: 10,
        pixel_height: 10,
        duration: None,
        has_audio: Some(true),
    });
    state.preview.kind = PreviewKind::Video;
    state.preview.metadata_viewport.visible_rows = 2;
    state.preview.playback = PlaybackState::Playing;

    assert!(update(&mut state, Action::PreviewMetadataPageDown).is_empty());
    assert_eq!(state.preview.playback, PlaybackState::Playing);
    assert!(state.preview.metadata_viewport.offset > 0);

    state.preview.playback = PlaybackState::Stopped;
    assert!(update(&mut state, Action::ToggleMute).is_empty());
    assert!(state.preview.muted);
}

#[test]
fn navigation_stops_video_and_still_executes_the_motion() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    state.preview.playback = PlaybackState::Playing;

    let effects = update(&mut state, Action::MoveSelection { delta: 1, count: 1 });
    assert_eq!(state.workspace.active().selected, 1);
    assert_eq!(state.preview.playback, PlaybackState::Stopped);
    assert!(matches!(effects.first(), Some(Effect::StopPreviewPlayback)));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::PreparePreviewMedia { path, .. } if path == &PathBuf::from("b.png")
    )));
}

#[test]
fn video_preview_stays_stopped_until_explicitly_toggled_and_restores_thumbnail() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    state.preview.kind = PreviewKind::Video;
    state.preview.media = LoadState::Ready(crate::preview::MediaMetadata {
        kind: PreviewKind::Video,
        pixel_width: 40,
        pixel_height: 20,
        duration: None,
        has_audio: Some(false),
    });

    assert_eq!(state.preview.playback, PlaybackState::Stopped);
    let effects = update(&mut state, Action::TogglePreviewPlayback);
    assert_eq!(state.preview.playback, PlaybackState::Starting);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::StartPreviewPlayback { .. }))
    );

    let generation = state.preview.generation;
    let path = state.preview.path.clone().unwrap();
    update(
        &mut state,
        Action::PreviewPlaybackStarted {
            generation,
            path,
            kind: PreviewKind::Video,
            has_audio: false,
        },
    );
    let effects = update(&mut state, Action::TogglePreviewPlayback);
    assert_eq!(state.preview.playback, PlaybackState::Stopped);
    assert!(matches!(effects.first(), Some(Effect::StopPreviewPlayback)));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::PreparePreviewMedia { .. }))
    );
}

#[test]
fn audio_preview_has_no_autoplay_and_each_toggle_starts_from_a_new_handle() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    let generation = state.preview.generation;
    let path = state.preview.path.clone().unwrap();
    update(
        &mut state,
        Action::PreviewMediaPrepared {
            generation,
            path: path.clone(),
            metadata: crate::preview::MediaMetadata {
                kind: PreviewKind::Audio,
                pixel_width: 0,
                pixel_height: 0,
                duration: Some(std::time::Duration::from_secs(2)),
                has_audio: Some(true),
            },
        },
    );
    assert_eq!(state.preview.playback, PlaybackState::Stopped);

    assert!(update(&mut state, Action::ToggleMute).is_empty());
    assert!(state.preview.muted);

    let first = update(&mut state, Action::TogglePreviewPlayback);
    assert!(matches!(
        first.last(),
        Some(Effect::StartPreviewPlayback { generation: value, path: effect_path, muted: true, .. })
            if *value == generation && effect_path == &path
    ));
    update(&mut state, Action::PreviewPlaybackStopped);
    let second = update(&mut state, Action::TogglePreviewPlayback);
    assert!(matches!(
        second.last(),
        Some(Effect::StartPreviewPlayback { generation: value, path: effect_path, .. })
            if *value == generation && effect_path == &path
    ));
}

#[test]
fn audio_snapshots_are_scoped_and_mute_does_not_stop_playback() {
    let mut state = state();
    update(&mut state, Action::TogglePreview);
    let generation = state.preview.generation;
    let path = state.preview.path.clone().unwrap();
    state.preview.kind = PreviewKind::Audio;
    state.preview.playback = PlaybackState::Playing;

    let effects = update(&mut state, Action::ToggleMute);
    assert!(state.preview.muted);
    assert_eq!(state.preview.playback, PlaybackState::Playing);
    assert!(matches!(
        effects.as_slice(),
        [Effect::SetPreviewMuted(true)]
    ));

    update(
        &mut state,
        Action::PreviewAudioSnapshot {
            generation: generation + 1,
            path: path.clone(),
            position: std::time::Duration::from_secs(9),
            muted: false,
            levels: vec![999; 64],
        },
    );
    assert_eq!(state.preview.playback_position, None);

    update(
        &mut state,
        Action::PreviewAudioSnapshot {
            generation,
            path,
            position: std::time::Duration::from_millis(250),
            muted: true,
            levels: vec![500; 64],
        },
    );
    assert_eq!(
        state.preview.playback_position,
        Some(std::time::Duration::from_millis(250))
    );
    assert_eq!(state.preview.audio_levels, vec![500; 64]);
    assert_eq!(state.preview.playback, PlaybackState::Playing);

    update(
        &mut state,
        Action::PreviewPlaybackFinished {
            generation: generation + 1,
            path: PathBuf::from("stale.mp3"),
        },
    );
    assert_eq!(state.preview.playback, PlaybackState::Playing);

    let current_path = state.preview.path.clone().unwrap();
    update(
        &mut state,
        Action::PreviewPlaybackFailed {
            generation,
            path: current_path,
            message: "decoder failed".into(),
        },
    );
    assert_eq!(state.preview.playback, PlaybackState::Stopped);
    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.level == crate::app::NotificationLevel::Error
            && notification.message.contains("decoder failed")
    }));
}
