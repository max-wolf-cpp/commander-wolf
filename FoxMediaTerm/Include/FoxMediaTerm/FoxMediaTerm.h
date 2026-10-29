#pragma once

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(FOX_MEDIA_TERM_BUILD)
#define FOX_MEDIA_TERM_API __declspec(dllexport)
#elif defined(_WIN32)
#define FOX_MEDIA_TERM_API __declspec(dllimport)
#elif defined(__GNUC__)
#define FOX_MEDIA_TERM_API __attribute__((visibility("default")))
#else
#define FOX_MEDIA_TERM_API
#endif

#ifdef __cplusplus
extern "C"
{
#endif
    typedef uint32_t FoxMediaTermStatus;
    enum
    {
        FOX_MEDIA_TERM_STATUS_OK = 0,
        FOX_MEDIA_TERM_STATUS_NO_FRAME = 1,
        FOX_MEDIA_TERM_STATUS_FINISHED = 2,
        FOX_MEDIA_TERM_STATUS_ERROR = 3,
    };

    typedef uint32_t FoxMediaTermProtocol;
    enum
    {
        FOX_MEDIA_TERM_PROTOCOL_AUTO = 0,
        FOX_MEDIA_TERM_PROTOCOL_ITERM2 = 1,
        FOX_MEDIA_TERM_PROTOCOL_SIXEL = 2,
        FOX_MEDIA_TERM_PROTOCOL_KITTY = 3,
        FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR = 4,
        FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_256 = 5,
        FOX_MEDIA_TERM_PROTOCOL_ASCII_GRAYSCALE = 6,
    };

    typedef uint32_t FoxMediaTermKind;
    enum
    {
        FOX_MEDIA_TERM_KIND_IMAGE = 1,
        FOX_MEDIA_TERM_KIND_VIDEO = 2,
        FOX_MEDIA_TERM_KIND_AUDIO = 3,
    };

    typedef uint32_t FoxMediaTermPixelFormat;
    enum
    {
        FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8 = 1,
    };

    enum
    {
        FOX_MEDIA_TERM_BUFFER_ANSI_UTF8 = 1U << 0U,
    };

    enum
    {
        FOX_MEDIA_TERM_MEDIA_INFO_DURATION_KNOWN = 1U << 0U,
        FOX_MEDIA_TERM_MEDIA_INFO_AUDIO_KNOWN = 1U << 1U,
        FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO = 1U << 2U,
    };

    typedef struct FoxMediaTermRegion
    {
        int32_t column;
        int32_t row;
        int32_t columns;
        int32_t rows;
    } FoxMediaTermRegion;

    typedef struct FoxMediaTermEnvironment
    {
        FoxMediaTermProtocol protocol;
        int32_t cellPixelWidth;
        int32_t cellPixelHeight;
        uint32_t exactCellPixelSize;
    } FoxMediaTermEnvironment;

    typedef struct FoxMediaTermMediaInfo
    {
        FoxMediaTermKind kind;
        int32_t pixelWidth;
        int32_t pixelHeight;
        int64_t durationMicroseconds;
        uint32_t flags;
    } FoxMediaTermMediaInfo;

    typedef struct FoxMediaTermBuffer
    {
        // Owned bytes remain valid until FoxMediaTermReleaseBuffer is called.
        const uint8_t* data;
        size_t size;
        // Opaque library-owned state. Callers must not inspect or free it.
        void* context;
        uint32_t flags;
    } FoxMediaTermBuffer;

    typedef struct FoxMediaTermError
    {
        // UTF-8 text remains valid until FoxMediaTermReleaseError is called.
        const char* data;
        size_t size;
        // Opaque library-owned state. Callers must not inspect or free it.
        void* context;
    } FoxMediaTermError;

    typedef struct FoxMediaTermVideo FoxMediaTermVideo;
    typedef struct FoxMediaTermAudio FoxMediaTermAudio;

    // Returns an empty buffer when the terminal does not support the request or is busy.
    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermQueryTerminal(
        const uint8_t* request,
        size_t requestSize,
        uint32_t timeoutMilliseconds,
        FoxMediaTermBuffer* response,
        FoxMediaTermError* error
    );

    enum
    {
        FOX_MEDIA_TERM_AUDIO_LEVEL_COUNT = 64,
    };

    typedef struct FoxMediaTermAudioUpdate
    {
        int64_t positionMicroseconds;
        uint32_t muted;
        uint32_t levelCount;
        float levels[FOX_MEDIA_TERM_AUDIO_LEVEL_COUNT];
    } FoxMediaTermAudioUpdate;

    // Paths are length-delimited UTF-8 and do not need a trailing null byte. No C++
    // exception crosses this ABI. Every returned buffer, error, and video handle must
    // be released with its matching function. A video handle has one consumer thread.

    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermDetectEnvironment(
        int32_t terminalColumns,
        int32_t terminalRows,
        FoxMediaTermEnvironment* environment,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermPreparePreview(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        FoxMediaTermKind* kind,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    );

    // The input pixels are copied before this function returns. Stride and buffer
    // size are expressed in bytes and must describe every complete source row.
    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermPrepareRaster(
        const uint8_t* pixels,
        size_t pixelsSize,
        int32_t width,
        int32_t height,
        size_t stride,
        FoxMediaTermPixelFormat pixelFormat,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermProbeMedia(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermMediaInfo* info,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermPrepareCleanup(
        FoxMediaTermProtocol protocol,
        FoxMediaTermRegion region,
        uint8_t backgroundRed,
        uint8_t backgroundGreen,
        uint8_t backgroundBlue,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API FoxMediaTermVideo* FoxMediaTermStartVideo(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        int32_t terminalColumns,
        int32_t terminalRows,
        uint32_t muted,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API FoxMediaTermStatus FoxMediaTermPollVideo(
        FoxMediaTermVideo* video,
        int32_t terminalColumns,
        int32_t terminalRows,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    );

    FOX_MEDIA_TERM_API void FoxMediaTermSetVideoMuted(FoxMediaTermVideo* video, uint32_t muted);
    FOX_MEDIA_TERM_API uint32_t FoxMediaTermIsVideoMuted(const FoxMediaTermVideo* video);
    FOX_MEDIA_TERM_API uint32_t FoxMediaTermVideoHasAudio(const FoxMediaTermVideo* video);
    FOX_MEDIA_TERM_API void FoxMediaTermDestroyVideo(FoxMediaTermVideo* video);

    FOX_MEDIA_TERM_API FoxMediaTermAudio* FoxMediaTermStartAudio(
        const char* pathUtf8,
        size_t pathSize,
        uint32_t muted,
        FoxMediaTermError* error
    );
    FOX_MEDIA_TERM_API FoxMediaTermStatus
    FoxMediaTermPollAudio(FoxMediaTermAudio* audio, FoxMediaTermAudioUpdate* update, FoxMediaTermError* error);
    FOX_MEDIA_TERM_API void FoxMediaTermSetAudioMuted(FoxMediaTermAudio* audio, uint32_t muted);
    FOX_MEDIA_TERM_API uint32_t FoxMediaTermIsAudioMuted(const FoxMediaTermAudio* audio);
    FOX_MEDIA_TERM_API void FoxMediaTermDestroyAudio(FoxMediaTermAudio* audio);

    FOX_MEDIA_TERM_API void FoxMediaTermReleaseBuffer(FoxMediaTermBuffer* buffer);
    FOX_MEDIA_TERM_API void FoxMediaTermReleaseError(FoxMediaTermError* error);

#ifdef __cplusplus
}
#endif
