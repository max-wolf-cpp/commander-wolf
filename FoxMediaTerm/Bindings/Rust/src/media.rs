use std::path::Path;
use std::time::Duration;

use crate::Error;
use crate::ffi::{self, NativeError, NativeMediaInfo};
use crate::native::{check_ok_status, path_utf8};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaKind {
    Image,
    Video,
    Audio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MediaInfo {
    pub kind: MediaKind,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub duration: Option<Duration>,
    pub has_audio: Option<bool>,
}

impl MediaKind {
    pub(super) fn from_raw(value: u32) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::Image),
            2 => Ok(Self::Video),
            3 => Ok(Self::Audio),
            _ => Err(Error::invalid_native_value("media kind", value)),
        }
    }
}

pub fn probe_media(path: &Path) -> Result<MediaInfo, Error> {
    let path_utf8 = path_utf8(path)?;
    let mut native = NativeMediaInfo::default();
    let mut error = NativeError::default();
    let status = unsafe {
        ffi::probe_media(
            path_utf8.as_ptr().cast(),
            path_utf8.len(),
            &mut native,
            &mut error,
        )
    };
    check_ok_status(status, &mut error)?;

    media_info_from_native(native)
}

fn media_info_from_native(native: NativeMediaInfo) -> Result<MediaInfo, Error> {
    if native.flags & !ffi::MEDIA_INFO_FLAGS != 0 {
        return Err(Error::invalid_native_value(
            "media info flags",
            native.flags,
        ));
    }

    let duration_known = native.flags & ffi::MEDIA_INFO_DURATION_KNOWN != 0;
    let audio_known = native.flags & ffi::MEDIA_INFO_AUDIO_KNOWN != 0;
    let has_audio = native.flags & ffi::MEDIA_INFO_HAS_AUDIO != 0;
    if has_audio && !audio_known {
        return Err(Error::new(
            "Fox Media Term returned audio presence without audio information",
        ));
    }

    if native.duration_microseconds < 0 {
        return Err(Error::new(
            "Fox Media Term returned a negative media duration",
        ));
    }

    if !duration_known && native.duration_microseconds != 0 {
        return Err(Error::new(
            "Fox Media Term returned a duration without the duration-known flag",
        ));
    }

    let kind = MediaKind::from_raw(native.kind)?;
    match kind {
        MediaKind::Audio if native.pixel_width != 0 || native.pixel_height != 0 => {
            return Err(Error::new(
                "Fox Media Term returned dimensions for audio-only media",
            ));
        }
        MediaKind::Image | MediaKind::Video
            if native.pixel_width <= 0 || native.pixel_height <= 0 =>
        {
            return Err(Error::new(
                "Fox Media Term returned invalid media dimensions",
            ));
        }
        _ => {}
    }

    if kind == MediaKind::Image && native.flags != 0 {
        return Err(Error::new(
            "Fox Media Term returned video metadata flags for an image",
        ));
    }

    if kind == MediaKind::Audio && (!audio_known || !has_audio) {
        return Err(Error::new(
            "Fox Media Term returned audio-only media without an audio stream",
        ));
    }

    Ok(MediaInfo {
        kind,
        pixel_width: native.pixel_width as u32,
        pixel_height: native.pixel_height as u32,
        duration: duration_known
            .then(|| Duration::from_micros(native.duration_microseconds as u64)),
        has_audio: audio_known.then_some(has_audio),
    })
}

#[cfg(test)]
mod tests {
    use super::{MediaKind, media_info_from_native};
    use crate::ffi::{
        MEDIA_INFO_AUDIO_KNOWN, MEDIA_INFO_DURATION_KNOWN, MEDIA_INFO_HAS_AUDIO, NativeMediaInfo,
    };

    #[test]
    fn validates_and_converts_native_media_information() {
        let info = media_info_from_native(NativeMediaInfo {
            kind: 2,
            pixel_width: 1920,
            pixel_height: 1080,
            duration_microseconds: 1_500_000,
            flags: MEDIA_INFO_DURATION_KNOWN | MEDIA_INFO_AUDIO_KNOWN,
        })
        .unwrap();
        assert_eq!(info.kind, MediaKind::Video);
        assert_eq!(info.duration.unwrap().as_millis(), 1500);
        assert_eq!(info.has_audio, Some(false));

        let with_audio = media_info_from_native(NativeMediaInfo {
            kind: 2,
            pixel_width: 4,
            pixel_height: 4,
            duration_microseconds: 0,
            flags: MEDIA_INFO_AUDIO_KNOWN | MEDIA_INFO_HAS_AUDIO,
        })
        .unwrap();
        assert_eq!(with_audio.has_audio, Some(true));

        let audio = media_info_from_native(NativeMediaInfo {
            kind: 3,
            pixel_width: 0,
            pixel_height: 0,
            duration_microseconds: 2_000_000,
            flags: MEDIA_INFO_DURATION_KNOWN | MEDIA_INFO_AUDIO_KNOWN | MEDIA_INFO_HAS_AUDIO,
        })
        .unwrap();
        assert_eq!(audio.kind, MediaKind::Audio);
        assert_eq!(audio.duration.unwrap().as_secs(), 2);

        assert!(
            media_info_from_native(NativeMediaInfo {
                kind: 2,
                pixel_width: 4,
                pixel_height: 4,
                duration_microseconds: 0,
                flags: MEDIA_INFO_HAS_AUDIO,
            })
            .is_err()
        );

        assert!(
            media_info_from_native(NativeMediaInfo {
                kind: 1,
                pixel_width: 4,
                pixel_height: 4,
                duration_microseconds: 1,
                flags: 0,
            })
            .is_err()
        );

        assert!(
            media_info_from_native(NativeMediaInfo {
                kind: 1,
                pixel_width: 0,
                pixel_height: 1,
                duration_microseconds: 0,
                flags: 0,
            })
            .is_err()
        );
    }
}
