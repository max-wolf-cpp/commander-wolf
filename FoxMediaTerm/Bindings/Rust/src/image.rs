use std::path::Path;

use crate::ffi::{self, NativeBuffer, NativeError};
use crate::native::{check_ok_status, path_utf8};
use crate::{Buffer, Error, MediaKind, Protocol, Region};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PixelFormat {
    Rgba8,
}

impl PixelFormat {
    const fn as_raw(self) -> u32 {
        match self {
            Self::Rgba8 => 1,
        }
    }

    const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8 => 4,
        }
    }
}

pub fn prepare_preview(
    path: &Path,
    protocol: Protocol,
    cell_pixel_width: i32,
    cell_pixel_height: i32,
    region: Region,
) -> Result<(MediaKind, Buffer), Error> {
    let path_utf8 = path_utf8(path)?;
    let mut kind = 0;
    let mut buffer = NativeBuffer::default();
    let mut error = NativeError::default();
    let status = unsafe {
        ffi::prepare_preview(
            path_utf8.as_ptr().cast(),
            path_utf8.len(),
            protocol.as_raw(),
            cell_pixel_width,
            cell_pixel_height,
            region.into(),
            &mut kind,
            &mut buffer,
            &mut error,
        )
    };
    if let Err(message) = check_ok_status(status, &mut error) {
        unsafe { ffi::release_buffer(&mut buffer) };
        return Err(message);
    }

    let buffer = Buffer::from_native(buffer);
    let kind = MediaKind::from_raw(kind)?;

    Ok((kind, buffer))
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_raster(
    pixels: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    pixel_format: PixelFormat,
    protocol: Protocol,
    cell_pixel_width: i32,
    cell_pixel_height: i32,
    region: Region,
) -> Result<Buffer, Error> {
    let width_i32 = i32::try_from(width).map_err(|_| Error::new("raster width is too large"))?;
    let height_i32 = i32::try_from(height).map_err(|_| Error::new("raster height is too large"))?;
    if width == 0 || height == 0 {
        return Err(Error::new("raster dimensions must be positive"));
    }

    let row_size = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(pixel_format.bytes_per_pixel()))
        .ok_or_else(|| Error::new("raster row size overflows"))?;
    if stride < row_size {
        return Err(Error::new("raster stride is shorter than one pixel row"));
    }

    let required = stride
        .checked_mul(height as usize)
        .ok_or_else(|| Error::new("raster buffer size overflows"))?;
    if pixels.len() < required {
        return Err(Error::new("raster pixel buffer is too short"));
    }

    let mut buffer = NativeBuffer::default();
    let mut error = NativeError::default();
    let status = unsafe {
        ffi::prepare_raster(
            pixels.as_ptr(),
            pixels.len(),
            width_i32,
            height_i32,
            stride,
            pixel_format.as_raw(),
            protocol.as_raw(),
            cell_pixel_width,
            cell_pixel_height,
            region.into(),
            &mut buffer,
            &mut error,
        )
    };
    if let Err(message) = check_ok_status(status, &mut error) {
        unsafe { ffi::release_buffer(&mut buffer) };
        return Err(message);
    }

    Ok(Buffer::from_native(buffer))
}

pub fn prepare_cleanup(
    protocol: Protocol,
    region: Region,
    background: [u8; 3],
) -> Result<Buffer, Error> {
    let mut buffer = NativeBuffer::default();
    let mut error = NativeError::default();
    let status = unsafe {
        ffi::prepare_cleanup(
            protocol.as_raw(),
            region.into(),
            background[0],
            background[1],
            background[2],
            &mut buffer,
            &mut error,
        )
    };
    if let Err(message) = check_ok_status(status, &mut error) {
        unsafe { ffi::release_buffer(&mut buffer) };
        return Err(message);
    }

    Ok(Buffer::from_native(buffer))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{PixelFormat, prepare_cleanup, prepare_preview, prepare_raster};
    use crate::{Protocol, Region};

    #[test]
    fn kitty_cleanup_removes_the_existing_graphics_layer() {
        let payload = prepare_cleanup(
            Protocol::Kitty,
            Region {
                column: 10,
                row: 5,
                columns: 20,
                rows: 8,
            },
            [0x18, 0x18, 0x25],
        )
        .expect("Kitty cleanup payload");

        assert!(
            payload
                .bytes()
                .windows(b"a=d,d=A".len())
                .any(|window| window == b"a=d,d=A")
        );
    }

    #[test]
    fn invalid_preview_path_returns_a_safe_rust_error() {
        let error = prepare_preview(
            Path::new("missing/Кириллица/狼.png"),
            Protocol::HalfBlockTrueColor,
            8,
            16,
            Region {
                column: 0,
                row: 0,
                columns: 20,
                rows: 10,
            },
        )
        .err()
        .expect("missing image must return an error");

        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn raster_wrapper_rejects_short_buffers_before_ffi() {
        let error = prepare_raster(
            &[0; 15],
            2,
            2,
            8,
            PixelFormat::Rgba8,
            Protocol::HalfBlockTrueColor,
            1,
            1,
            Region {
                column: 0,
                row: 0,
                columns: 2,
                rows: 2,
            },
        )
        .err()
        .expect("a short raster must return an error");
        assert!(error.to_string().contains("too short"));
    }

    #[test]
    fn raster_wrapper_prepares_a_small_owned_payload() {
        let payload = prepare_raster(
            &[255, 0, 0, 255, 0, 255, 0, 255],
            2,
            1,
            8,
            PixelFormat::Rgba8,
            Protocol::HalfBlockTrueColor,
            1,
            1,
            Region {
                column: 0,
                row: 0,
                columns: 2,
                rows: 1,
            },
        )
        .expect("small raster payload");
        assert!(!payload.bytes().is_empty());
    }
}
