use std::path::PathBuf;
use std::time::Instant;

use fox_media_term::{Audio, Environment, MediaInfo, MediaKind, Region, Video};

use crate::preview::{MediaMetadata, PreviewKind};
use crate::workspace::PanelId;

mod playback;
mod presentation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaSurface {
    Preview { panel: PanelId, region: Region },
    Viewer { region: Region },
}

pub struct MediaRuntime {
    environment: Environment,
    video: Option<Video>,
    audio: Option<Audio>,
    playback_context: Option<(u64, PathBuf)>,
    last_audio_snapshot: Option<Instant>,
    surface: Option<MediaSurface>,
}

impl MediaRuntime {
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            video: None,
            audio: None,
            playback_context: None,
            last_audio_snapshot: None,
            surface: None,
        }
    }

    pub fn protocol_label(&self) -> String {
        format!(
            "{} · {}x{} px{}",
            self.environment.protocol.label(),
            self.environment.cell_pixel_width,
            self.environment.cell_pixel_height,
            if self.environment.exact_cell_pixel_size {
                ""
            } else {
                " estimated"
            }
        )
    }
}

fn media_metadata(info: MediaInfo) -> MediaMetadata {
    MediaMetadata {
        kind: match info.kind {
            MediaKind::Image => PreviewKind::Image,
            MediaKind::Video => PreviewKind::Video,
            MediaKind::Audio => PreviewKind::Audio,
        },
        pixel_width: info.pixel_width,
        pixel_height: info.pixel_height,
        duration: info.duration,
        has_audio: info.has_audio,
    }
}
