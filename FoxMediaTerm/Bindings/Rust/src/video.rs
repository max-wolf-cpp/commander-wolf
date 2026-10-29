use std::path::Path;
use std::ptr::NonNull;

use crate::ffi::{self, NativeBuffer, NativeError, NativeVideo};
use crate::native::{path_utf8, release_error, release_native_buffer, take_error};
use crate::{Buffer, Error, Protocol, Region};

pub enum PollResult {
    Frame(Buffer),
    NoFrame,
    Finished,
}

pub struct Video {
    native: NonNull<NativeVideo>,
}

impl Video {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        path: &Path,
        protocol: Protocol,
        cell_pixel_width: i32,
        cell_pixel_height: i32,
        region: Region,
        terminal_columns: i32,
        terminal_rows: i32,
        muted: bool,
    ) -> Result<Self, Error> {
        let path_utf8 = path_utf8(path)?;
        let mut error = NativeError::default();
        let native = unsafe {
            ffi::start_video(
                path_utf8.as_ptr().cast(),
                path_utf8.len(),
                protocol.as_raw(),
                cell_pixel_width,
                cell_pixel_height,
                region.into(),
                terminal_columns,
                terminal_rows,
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

    pub fn poll(&mut self, terminal_columns: i32, terminal_rows: i32) -> Result<PollResult, Error> {
        let mut buffer = NativeBuffer::default();
        let mut error = NativeError::default();
        let status = unsafe {
            ffi::poll_video(
                self.native.as_ptr(),
                terminal_columns,
                terminal_rows,
                &mut buffer,
                &mut error,
            )
        };

        match status {
            ffi::STATUS_OK => {
                release_error(&mut error);
                Ok(PollResult::Frame(Buffer::from_native(buffer)))
            }
            ffi::STATUS_NO_FRAME => {
                release_native_buffer(&mut buffer);
                release_error(&mut error);
                Ok(PollResult::NoFrame)
            }
            ffi::STATUS_FINISHED => {
                release_native_buffer(&mut buffer);
                release_error(&mut error);
                Ok(PollResult::Finished)
            }
            ffi::STATUS_ERROR => {
                release_native_buffer(&mut buffer);
                Err(take_error(&mut error))
            }
            value => {
                release_native_buffer(&mut buffer);
                release_error(&mut error);
                Err(Error::invalid_native_value("status", value))
            }
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        unsafe { ffi::set_video_muted(self.native.as_ptr(), u32::from(muted)) };
    }

    pub fn is_muted(&self) -> bool {
        unsafe { ffi::is_video_muted(self.native.as_ptr()) != 0 }
    }

    pub fn has_audio(&self) -> bool {
        unsafe { ffi::video_has_audio(self.native.as_ptr()) != 0 }
    }
}

impl Drop for Video {
    fn drop(&mut self) {
        unsafe { ffi::destroy_video(self.native.as_ptr()) };
    }
}
