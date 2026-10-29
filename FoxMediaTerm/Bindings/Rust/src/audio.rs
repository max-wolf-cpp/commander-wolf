use std::path::Path;
use std::ptr::NonNull;
use std::time::Duration;

use crate::Error;
use crate::ffi::{self, NativeAudio, NativeAudioUpdate, NativeError};
use crate::native::{path_utf8, release_error, take_error};

#[derive(Clone, Debug, PartialEq)]
pub struct AudioUpdate {
    pub position: Duration,
    pub muted: bool,
    pub levels: [f32; ffi::AUDIO_LEVEL_COUNT],
}

pub enum AudioPollResult {
    Playing(AudioUpdate),
    Finished(AudioUpdate),
}

pub struct Audio {
    native: NonNull<NativeAudio>,
}

impl Audio {
    pub fn start(path: &Path, muted: bool) -> Result<Self, Error> {
        let path_utf8 = path_utf8(path)?;
        let mut error = NativeError::default();
        let native = unsafe {
            ffi::start_audio(
                path_utf8.as_ptr().cast(),
                path_utf8.len(),
                u32::from(muted),
                &mut error,
            )
        };

        match NonNull::new(native) {
            Some(native) => {
                release_error(&mut error);
                Ok(Self { native })
            }
            None => Err(take_error(&mut error)),
        }
    }

    pub fn poll(&mut self) -> Result<AudioPollResult, Error> {
        let mut update = NativeAudioUpdate::default();
        let mut error = NativeError::default();
        let status = unsafe { ffi::poll_audio(self.native.as_ptr(), &mut update, &mut error) };

        match status {
            ffi::STATUS_OK => {
                release_error(&mut error);
                Ok(AudioPollResult::Playing(audio_update_from_native(update)?))
            }
            ffi::STATUS_FINISHED => {
                release_error(&mut error);
                Ok(AudioPollResult::Finished(audio_update_from_native(update)?))
            }
            ffi::STATUS_ERROR => Err(take_error(&mut error)),
            value => {
                release_error(&mut error);
                Err(Error::invalid_native_value("status", value))
            }
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        unsafe { ffi::set_audio_muted(self.native.as_ptr(), u32::from(muted)) };
    }

    pub fn is_muted(&self) -> bool {
        unsafe { ffi::is_audio_muted(self.native.as_ptr()) != 0 }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        unsafe { ffi::destroy_audio(self.native.as_ptr()) };
    }
}

fn audio_update_from_native(native: NativeAudioUpdate) -> Result<AudioUpdate, Error> {
    if native.position_microseconds < 0 {
        return Err(Error::new(
            "Fox Media Term returned a negative audio position",
        ));
    }

    if native.level_count as usize != ffi::AUDIO_LEVEL_COUNT {
        return Err(Error::invalid_native_value(
            "audio visualization level count",
            native.level_count,
        ));
    }

    if native
        .levels
        .iter()
        .any(|level| !level.is_finite() || !(0.0..=1.0).contains(level))
    {
        return Err(Error::new(
            "Fox Media Term returned an invalid audio visualization level",
        ));
    }

    Ok(AudioUpdate {
        position: Duration::from_micros(native.position_microseconds as u64),
        muted: native.muted != 0,
        levels: native.levels,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Audio;

    #[test]
    fn audio_start_failure_is_reported_without_leaking_a_native_handle() {
        let error = Audio::start(Path::new("missing/audio/track.mp3"), false)
            .err()
            .expect("missing audio must return an error");
        assert!(!error.to_string().is_empty());
    }
}
