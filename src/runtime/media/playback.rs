use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fox_media_term::{Audio, AudioPollResult, MediaKind, PollResult, Region, Video};

use crate::app::Action;
use crate::terminal::TerminalSize;
use crate::workspace::PanelId;

use super::super::terminal_writer::TerminalWriter;
use super::{MediaRuntime, MediaSurface, media_metadata};

impl MediaRuntime {
    pub fn start(
        &mut self,
        generation: u64,
        path: &Path,
        size: TerminalSize,
        muted: bool,
        panel: PanelId,
        region: Region,
    ) -> Result<Action, String> {
        let info = fox_media_term::probe_media(path).map_err(|error| error.to_string())?;

        self.video = None;
        self.audio = None;
        self.playback_context = Some((generation, path.to_path_buf()));
        self.last_audio_snapshot = None;

        let kind = media_metadata(info).kind;
        let has_audio = match info.kind {
            MediaKind::Video => {
                let video = Video::start(
                    path,
                    self.environment.protocol,
                    self.environment.cell_pixel_width,
                    self.environment.cell_pixel_height,
                    region,
                    i32::from(size.columns),
                    i32::from(size.rows),
                    muted,
                )
                .map_err(|error| error.to_string())?;
                let has_audio = video.has_audio();
                self.video = Some(video);
                self.surface = Some(MediaSurface::Preview { panel, region });
                has_audio
            }
            MediaKind::Audio => {
                self.audio = Some(Audio::start(path, muted).map_err(|error| error.to_string())?);
                self.surface = None;
                true
            }
            MediaKind::Image => return Err("Images do not support preview playback".into()),
        };

        Ok(Action::PreviewPlaybackStarted {
            generation,
            path: path.to_path_buf(),
            kind,
            has_audio,
        })
    }

    pub fn stop(&mut self) {
        self.video = None;
        self.audio = None;
        self.playback_context = None;
        self.last_audio_snapshot = None;
    }

    pub fn set_muted(&mut self, muted: bool) {
        if let Some(video) = self.video.as_mut() {
            video.set_muted(muted);
        }
        if let Some(audio) = self.audio.as_mut() {
            audio.set_muted(muted);
        }
    }

    pub fn poll(&mut self, size: TerminalSize, writer: &mut TerminalWriter) -> Option<Action> {
        let (generation, path) = self.playback_context.clone()?;
        if self.video.is_some() {
            return self.poll_video(generation, path, size, writer);
        }

        self.poll_audio(generation, path)
    }

    fn poll_video(
        &mut self,
        generation: u64,
        path: PathBuf,
        size: TerminalSize,
        writer: &mut TerminalWriter,
    ) -> Option<Action> {
        match self
            .video
            .as_mut()?
            .poll(i32::from(size.columns), i32::from(size.rows))
        {
            Ok(PollResult::Frame(payload)) => {
                writer
                    .write_media(&payload)
                    .err()
                    .map(|message| Action::PreviewPlaybackFailed {
                        generation,
                        path,
                        message,
                    })
            }
            Ok(PollResult::NoFrame) => None,
            Ok(PollResult::Finished) => {
                self.video = None;
                self.playback_context = None;
                Some(Action::PreviewPlaybackFinished { generation, path })
            }
            Err(error) => {
                self.video = None;
                self.playback_context = None;
                Some(Action::PreviewPlaybackFailed {
                    generation,
                    path,
                    message: error.to_string(),
                })
            }
        }
    }

    fn poll_audio(&mut self, generation: u64, path: PathBuf) -> Option<Action> {
        let audio = self.audio.as_mut()?;
        if self
            .last_audio_snapshot
            .is_some_and(|last| last.elapsed() < Duration::from_millis(33))
        {
            return None;
        }

        self.last_audio_snapshot = Some(Instant::now());
        match audio.poll() {
            Ok(AudioPollResult::Playing(update)) => Some(Action::PreviewAudioSnapshot {
                generation,
                path,
                position: update.position,
                muted: update.muted,
                levels: normalized_levels(&update.levels),
            }),
            Ok(AudioPollResult::Finished(_)) => {
                self.audio = None;
                self.playback_context = None;
                Some(Action::PreviewPlaybackFinished { generation, path })
            }
            Err(error) => {
                self.audio = None;
                self.playback_context = None;
                Some(Action::PreviewPlaybackFailed {
                    generation,
                    path,
                    message: error.to_string(),
                })
            }
        }
    }
}

fn normalized_levels(levels: &[f32]) -> Vec<u16> {
    levels
        .iter()
        .map(|level| (level.clamp(0.0, 1.0) * 1_000.0).round() as u16)
        .collect()
}
