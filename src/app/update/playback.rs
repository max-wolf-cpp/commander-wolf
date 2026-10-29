use super::preview::{
    clamp_metadata_viewport, prepare_selected, preview_media_region, restore_preview_media,
};
use crate::app::{Action, AppState, Effect, Notification};
use crate::preview::{LoadState, PlaybackState, PreviewKind};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::TogglePreviewPlayback => toggle_preview_playback(state),
        Action::ToggleMute => toggle_mute(state),
        Action::PreviewPlaybackStarted {
            generation,
            path,
            kind,
            has_audio,
        } => {
            if state.preview.matches(generation, &path) {
                state.preview.kind = kind;
                state.preview.playback = PlaybackState::Playing;
                state.preview.surface_suspended = false;
                state.preview.has_audio = Some(has_audio);
                state.preview.playback_position = Some(std::time::Duration::ZERO);

                if let LoadState::Ready(metadata) = &mut state.preview.media {
                    metadata.has_audio = Some(has_audio);
                }

                state.notification = Some(Notification::info(match kind {
                    PreviewKind::Audio => "audio playing",
                    PreviewKind::Video if has_audio => "video playing with audio",
                    PreviewKind::Video => "video playing",
                    _ => "preview playing",
                }));
            }
            Vec::new()
        }
        Action::PreviewPlaybackStopped => {
            reset_playback(state);
            Vec::new()
        }
        Action::PreviewAudioSnapshot {
            generation,
            path,
            position,
            muted,
            levels,
        } => {
            if state.preview.matches(generation, &path)
                && state.preview.kind == PreviewKind::Audio
                && state.preview.playback == PlaybackState::Playing
            {
                state.preview.playback_position = Some(position);
                state.preview.muted = muted;
                state.preview.audio_levels = levels;
                clamp_metadata_viewport(state);
            }
            Vec::new()
        }
        Action::PreviewPlaybackFinished { generation, path } => {
            if !state.preview.matches(generation, &path) {
                return Vec::new();
            }

            let kind = state.preview.kind;
            reset_playback(state);
            state.notification = Some(Notification::info(match kind {
                PreviewKind::Audio => "audio playback finished",
                _ => "video playback finished",
            }));

            prepare_selected(state)
        }
        Action::PreviewPlaybackFailed {
            generation,
            path,
            message,
        } => {
            if !state.preview.matches(generation, &path) {
                return Vec::new();
            }

            reset_playback(state);
            state.notification = Some(Notification::error(format!(
                "Preview playback failed: {message}"
            )));

            prepare_selected(state)
        }
        _ => unreachable!("dispatcher routed an unsupported action to playback"),
    }
}

fn toggle_preview_playback(state: &mut AppState) -> Vec<Effect> {
    if !state.workspace.preview_visible {
        return Vec::new();
    }

    if state.is_preview_playing() {
        reset_playback(state);

        let mut effects = vec![Effect::StopPreviewPlayback, Effect::ClearMediaSurface];
        effects.extend(restore_preview_media(state));
        effects
    } else if state.preview_is_playable() {
        state.preview.playback = PlaybackState::Starting;
        let panel = state.workspace.active_panel.other();
        let region = preview_media_region(state, panel);

        let mut effects = vec![Effect::ClearMediaSurface];
        effects.extend(
            state
                .selected_path()
                .map(|path| Effect::StartPreviewPlayback {
                    generation: state.preview.generation,
                    path: path.to_path_buf(),
                    size: state.terminal_size,
                    muted: state.preview.muted,
                    panel,
                    region,
                }),
        );
        effects
    } else {
        Vec::new()
    }
}

pub(super) fn stop_video(state: &mut AppState) -> Vec<Effect> {
    if matches!(
        state.preview.playback,
        PlaybackState::Starting | PlaybackState::Playing
    ) {
        reset_playback(state);
        vec![Effect::StopPreviewPlayback]
    } else {
        Vec::new()
    }
}

fn toggle_mute(state: &mut AppState) -> Vec<Effect> {
    if !state.preview_is_playable() {
        return Vec::new();
    }

    state.preview.muted = !state.preview.muted;
    let media = if state.preview.kind == PreviewKind::Audio {
        "audio"
    } else {
        "video"
    };
    let status = if state.preview.muted {
        "muted"
    } else {
        "unmuted"
    };
    state.notification = Some(Notification::info(format!("{media} {status}")));

    if state.is_preview_playing() {
        vec![Effect::SetPreviewMuted(state.preview.muted)]
    } else {
        Vec::new()
    }
}

fn reset_playback(state: &mut AppState) {
    state.preview.playback = PlaybackState::Stopped;
    state.preview.playback_position = None;
    state.preview.audio_levels.fill(0);
}
