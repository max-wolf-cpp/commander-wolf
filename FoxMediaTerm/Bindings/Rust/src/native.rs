use std::fmt;
use std::path::Path;
use std::slice;

use crate::ffi::{self, NativeBuffer, NativeError};

#[derive(Debug, Eq, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(super) fn invalid_native_value(kind: &str, value: u32) -> Self {
        Self::new(format!(
            "Fox Media Term returned an invalid {kind} value: {value}"
        ))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub struct Buffer {
    native: NativeBuffer,
}

impl Buffer {
    pub(super) fn from_native(native: NativeBuffer) -> Self {
        Self { native }
    }

    pub fn bytes(&self) -> &[u8] {
        if self.native.data.is_null() || self.native.size == 0 {
            return &[];
        }

        unsafe { slice::from_raw_parts(self.native.data, self.native.size) }
    }

    pub fn uses_ansi_utf8(&self) -> bool {
        self.native.flags & ffi::BUFFER_ANSI_UTF8 != 0
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { ffi::release_buffer(&mut self.native) };
    }
}

pub(super) fn path_utf8(path: &Path) -> Result<Vec<u8>, Error> {
    path.to_str()
        .map(|value| value.as_bytes().to_vec())
        .ok_or_else(|| Error::new(format!("path is not valid UTF-8: {}", path.display())))
}

pub(super) fn check_ok_status(status: u32, error: &mut NativeError) -> Result<(), Error> {
    match status {
        ffi::STATUS_OK => {
            release_error(error);
            Ok(())
        }
        ffi::STATUS_ERROR => Err(take_error(error)),
        value => {
            release_error(error);
            Err(Error::invalid_native_value("status", value))
        }
    }
}

pub(super) fn release_native_buffer(buffer: &mut NativeBuffer) {
    unsafe { ffi::release_buffer(buffer) };
}

pub(super) fn release_error(error: &mut NativeError) {
    unsafe { ffi::release_error(error) };
}

pub(super) fn take_error(error: &mut NativeError) -> Error {
    let message = if error.data.is_null() || error.size == 0 {
        "Fox Media Term returned an unspecified error".to_owned()
    } else {
        let bytes = unsafe { slice::from_raw_parts(error.data.cast::<u8>(), error.size) };
        String::from_utf8_lossy(bytes).into_owned()
    };
    release_error(error);

    Error::new(message)
}
