#pragma once

#include <cstdint>
#include <string_view>

extern "C"
{
#include <libavutil/channel_layout.h>
#include <libavutil/rational.h>
#include <libavutil/samplefmt.h>
}

#include "AudioDecoder.h"

struct AVCodec;
struct AVCodecContext;
struct AVFormatContext;
struct AVFrame;
struct AVPacket;
struct AVStream;
struct SwrContext;

namespace FoxMediaTerm
{
    struct AudioDecoder::Implementation
    {
        ~Implementation();

        [[nodiscard]] std::expected<void, std::string> OpenInput(const std::filesystem::path& path);
        [[nodiscard]] std::expected<const AVCodec*, std::string> SelectAudioStream();
        [[nodiscard]] std::expected<void, std::string> OpenCodec(const AVCodec* decoder);
        [[nodiscard]] std::expected<bool, std::string> ReceiveDecodedFrame();
        [[nodiscard]] std::expected<std::optional<AudioPcmChunk>, std::string> ConvertFrame();
        [[nodiscard]] std::expected<std::optional<AudioPcmChunk>, std::string> DrainResampler();

        [[nodiscard]] int ReadAudioPacket();
        [[nodiscard]] std::expected<void, std::string> EnsureResampler();
        [[nodiscard]] std::chrono::microseconds ReadTimestamp();
        [[nodiscard]] static std::string FfmpegError(std::string_view operation, int errorCode);
        [[nodiscard]] static std::chrono::microseconds RescaleTimestamp(std::int64_t timestamp, AVRational timeBase);

        AVFormatContext* formatContext = nullptr;
        AVCodecContext* codecContext = nullptr;
        AVFrame* decodedFrame = nullptr;
        AVPacket* packet = nullptr;
        SwrContext* resampleContext = nullptr;
        AVStream* audioStream = nullptr;
        AVChannelLayout inputLayout { };
        AVSampleFormat inputFormat = AV_SAMPLE_FMT_NONE;
        int inputSampleRate = 0;
        int audioStreamIndex = -1;
        bool drainPacketSent = false;
        bool resamplerDrained = false;
        std::chrono::microseconds fallbackTimestamp { 0 };
        std::optional<std::chrono::microseconds> streamStartTimestamp;
    };
} // namespace FoxMediaTerm
