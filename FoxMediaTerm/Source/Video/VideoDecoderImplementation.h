#pragma once

#include "VideoDecoder.h"

extern "C"
{
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libswscale/swscale.h>
}

namespace FoxMediaTerm
{
    // Shared decoder state stays private; per-frame operations remain in one translation unit.
    struct VideoDecoder::Implementation
    {
        ~Implementation();

        [[nodiscard]] std::expected<const AVCodec*, std::string> OpenInput(const std::filesystem::path& path);
        [[nodiscard]] std::expected<void, std::string> OpenCodec(const AVCodec* decoder);

        [[nodiscard]] std::expected<bool, std::string> ReceiveDecodedFrame();
        [[nodiscard]] std::chrono::microseconds ReadTimestamp();
        [[nodiscard]] std::expected<RasterImage, std::string> ConvertFrame(int maximumWidth, int maximumHeight);

        AVFormatContext* formatContext = nullptr;
        AVCodecContext* codecContext = nullptr;
        AVFrame* decodedFrame = nullptr;
        AVPacket* packet = nullptr;
        SwsContext* scaleContext = nullptr;
        AVStream* videoStream = nullptr;
        int videoStreamIndex = -1;
        bool drainPacketSent = false;
        std::chrono::microseconds fallbackTimestamp { 0 };
        std::chrono::microseconds frameDuration { 33'333 };
        std::optional<std::chrono::microseconds> lastTimestamp;
        std::uint64_t nextFrameIndex = 0;
        VideoPlaybackTelemetry* telemetry = nullptr;
    };
} // namespace FoxMediaTerm
