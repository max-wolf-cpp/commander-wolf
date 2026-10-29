use std::time::Duration;

use crate::ffi::{self, NativeBuffer, NativeEnvironment, NativeError, NativeRegion};
use crate::native::check_ok_status;
use crate::{Buffer, Error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    Auto,
    ITerm2,
    Sixel,
    Kitty,
    HalfBlockTrueColor,
    HalfBlock256,
    AsciiGrayscale,
}

impl Protocol {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "auto" => Ok(Self::Auto),
            "it2" => Ok(Self::ITerm2),
            "sxl" => Ok(Self::Sixel),
            "kty" => Ok(Self::Kitty),
            "hb-tc" => Ok(Self::HalfBlockTrueColor),
            "hb-256" => Ok(Self::HalfBlock256),
            "ag" => Ok(Self::AsciiGrayscale),
            _ => Err(Error::new(format!("unknown media protocol: {value}"))),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::ITerm2 => "iTerm2",
            Self::Sixel => "Sixel",
            Self::Kitty => "Kitty",
            Self::HalfBlockTrueColor => "HalfBlock TrueColor",
            Self::HalfBlock256 => "HalfBlock 256",
            Self::AsciiGrayscale => "ASCII grayscale",
        }
    }

    fn from_raw(value: u32) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Auto),
            1 => Ok(Self::ITerm2),
            2 => Ok(Self::Sixel),
            3 => Ok(Self::Kitty),
            4 => Ok(Self::HalfBlockTrueColor),
            5 => Ok(Self::HalfBlock256),
            6 => Ok(Self::AsciiGrayscale),
            _ => Err(Error::invalid_native_value("protocol", value)),
        }
    }

    pub(super) const fn as_raw(self) -> u32 {
        match self {
            Self::Auto => 0,
            Self::ITerm2 => 1,
            Self::Sixel => 2,
            Self::Kitty => 3,
            Self::HalfBlockTrueColor => 4,
            Self::HalfBlock256 => 5,
            Self::AsciiGrayscale => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Region {
    pub column: i32,
    pub row: i32,
    pub columns: i32,
    pub rows: i32,
}

impl From<Region> for NativeRegion {
    fn from(region: Region) -> Self {
        Self {
            column: region.column,
            row: region.row,
            columns: region.columns,
            rows: region.rows,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Environment {
    pub protocol: Protocol,
    pub cell_pixel_width: i32,
    pub cell_pixel_height: i32,
    pub exact_cell_pixel_size: bool,
}

impl Environment {
    pub fn detect(terminal_columns: i32, terminal_rows: i32) -> Result<Self, Error> {
        let mut native = NativeEnvironment::default();
        let mut error = NativeError::default();
        let status = unsafe {
            ffi::detect_environment(terminal_columns, terminal_rows, &mut native, &mut error)
        };
        check_ok_status(status, &mut error)?;

        Ok(Self {
            protocol: Protocol::from_raw(native.protocol)?,
            cell_pixel_width: native.cell_pixel_width,
            cell_pixel_height: native.cell_pixel_height,
            exact_cell_pixel_size: native.exact_cell_pixel_size != 0,
        })
    }
}

/// Queries an interactive terminal. An empty response means the terminal did not answer.
pub fn query_terminal(request: &[u8], timeout: Duration) -> Result<Buffer, Error> {
    let mut response = NativeBuffer::default();
    let mut error = NativeError::default();
    let milliseconds = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    let status = unsafe {
        ffi::query_terminal(
            request.as_ptr(),
            request.len(),
            milliseconds,
            &mut response,
            &mut error,
        )
    };
    check_ok_status(status, &mut error)?;

    Ok(Buffer::from_native(response))
}

#[cfg(test)]
mod tests {
    #[test]
    fn terminal_query_rejects_empty_requests() {
        let error = super::query_terminal(&[], std::time::Duration::from_millis(10))
            .err()
            .expect("an empty terminal request must fail");
        assert!(error.to_string().contains("terminal query"));
    }
}
