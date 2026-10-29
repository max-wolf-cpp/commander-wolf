use std::ffi::{c_char, c_void};

pub(super) const STATUS_OK: u32 = 0;
pub(super) const STATUS_NO_FRAME: u32 = 1;
pub(super) const STATUS_FINISHED: u32 = 2;
pub(super) const STATUS_ERROR: u32 = 3;
pub(super) const BUFFER_ANSI_UTF8: u32 = 1;
pub(super) const MEDIA_INFO_DURATION_KNOWN: u32 = 1 << 0;
pub(super) const MEDIA_INFO_AUDIO_KNOWN: u32 = 1 << 1;
pub(super) const MEDIA_INFO_HAS_AUDIO: u32 = 1 << 2;
pub(super) const MEDIA_INFO_FLAGS: u32 =
    MEDIA_INFO_DURATION_KNOWN | MEDIA_INFO_AUDIO_KNOWN | MEDIA_INFO_HAS_AUDIO;
pub(super) const AUDIO_LEVEL_COUNT: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct NativeRegion {
    pub(super) column: i32,
    pub(super) row: i32,
    pub(super) columns: i32,
    pub(super) rows: i32,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct NativeEnvironment {
    pub(super) protocol: u32,
    pub(super) cell_pixel_width: i32,
    pub(super) cell_pixel_height: i32,
    pub(super) exact_cell_pixel_size: u32,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct NativeMediaInfo {
    pub(super) kind: u32,
    pub(super) pixel_width: i32,
    pub(super) pixel_height: i32,
    pub(super) duration_microseconds: i64,
    pub(super) flags: u32,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct NativeBuffer {
    pub(super) data: *const u8,
    pub(super) size: usize,
    pub(super) context: *mut c_void,
    pub(super) flags: u32,
}

#[repr(C)]
#[derive(Default)]
pub(super) struct NativeError {
    pub(super) data: *const c_char,
    pub(super) size: usize,
    pub(super) context: *mut c_void,
}

#[repr(C)]
pub(super) struct NativeVideo {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct NativeAudio {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct NativeAudioUpdate {
    pub(super) position_microseconds: i64,
    pub(super) muted: u32,
    pub(super) level_count: u32,
    pub(super) levels: [f32; AUDIO_LEVEL_COUNT],
}

impl Default for NativeAudioUpdate {
    fn default() -> Self {
        Self {
            position_microseconds: 0,
            muted: 0,
            level_count: 0,
            levels: [0.0; AUDIO_LEVEL_COUNT],
        }
    }
}

#[link(name = "fox_media_term")]
unsafe extern "C" {
    #[link_name = "FoxMediaTermQueryTerminal"]
    pub(super) fn query_terminal(
        request: *const u8,
        request_size: usize,
        timeout_milliseconds: u32,
        response: *mut NativeBuffer,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermDetectEnvironment"]
    pub(super) fn detect_environment(
        terminal_columns: i32,
        terminal_rows: i32,
        environment: *mut NativeEnvironment,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermPreparePreview"]
    pub(super) fn prepare_preview(
        path_utf8: *const c_char,
        path_size: usize,
        protocol: u32,
        cell_pixel_width: i32,
        cell_pixel_height: i32,
        region: NativeRegion,
        kind: *mut u32,
        buffer: *mut NativeBuffer,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermPrepareRaster"]
    pub(super) fn prepare_raster(
        pixels: *const u8,
        pixels_size: usize,
        width: i32,
        height: i32,
        stride: usize,
        pixel_format: u32,
        protocol: u32,
        cell_pixel_width: i32,
        cell_pixel_height: i32,
        region: NativeRegion,
        buffer: *mut NativeBuffer,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermProbeMedia"]
    pub(super) fn probe_media(
        path_utf8: *const c_char,
        path_size: usize,
        info: *mut NativeMediaInfo,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermPrepareCleanup"]
    pub(super) fn prepare_cleanup(
        protocol: u32,
        region: NativeRegion,
        background_red: u8,
        background_green: u8,
        background_blue: u8,
        buffer: *mut NativeBuffer,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermStartVideo"]
    pub(super) fn start_video(
        path_utf8: *const c_char,
        path_size: usize,
        protocol: u32,
        cell_pixel_width: i32,
        cell_pixel_height: i32,
        region: NativeRegion,
        terminal_columns: i32,
        terminal_rows: i32,
        muted: u32,
        error: *mut NativeError,
    ) -> *mut NativeVideo;

    #[link_name = "FoxMediaTermPollVideo"]
    pub(super) fn poll_video(
        video: *mut NativeVideo,
        terminal_columns: i32,
        terminal_rows: i32,
        buffer: *mut NativeBuffer,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermSetVideoMuted"]
    pub(super) fn set_video_muted(video: *mut NativeVideo, muted: u32);

    #[link_name = "FoxMediaTermIsVideoMuted"]
    pub(super) fn is_video_muted(video: *const NativeVideo) -> u32;

    #[link_name = "FoxMediaTermVideoHasAudio"]
    pub(super) fn video_has_audio(video: *const NativeVideo) -> u32;

    #[link_name = "FoxMediaTermDestroyVideo"]
    pub(super) fn destroy_video(video: *mut NativeVideo);

    #[link_name = "FoxMediaTermStartAudio"]
    pub(super) fn start_audio(
        path_utf8: *const c_char,
        path_size: usize,
        muted: u32,
        error: *mut NativeError,
    ) -> *mut NativeAudio;

    #[link_name = "FoxMediaTermPollAudio"]
    pub(super) fn poll_audio(
        audio: *mut NativeAudio,
        update: *mut NativeAudioUpdate,
        error: *mut NativeError,
    ) -> u32;

    #[link_name = "FoxMediaTermSetAudioMuted"]
    pub(super) fn set_audio_muted(audio: *mut NativeAudio, muted: u32);

    #[link_name = "FoxMediaTermIsAudioMuted"]
    pub(super) fn is_audio_muted(audio: *const NativeAudio) -> u32;

    #[link_name = "FoxMediaTermDestroyAudio"]
    pub(super) fn destroy_audio(audio: *mut NativeAudio);

    #[link_name = "FoxMediaTermReleaseBuffer"]
    pub(super) fn release_buffer(buffer: *mut NativeBuffer);

    #[link_name = "FoxMediaTermReleaseError"]
    pub(super) fn release_error(error: *mut NativeError);
}
