#include <array>
#include <chrono>
#include <cstdint>
#include <memory>
#include <string>
#include <string_view>
#include <utility>

#include "VideoDecoder.h"
#include "VideoDecoderImplementation.h"
#include "VideoPlaybackTelemetry.h"

extern "C"
{
#include <libavutil/error.h>
#include <libavutil/log.h>
#include <libavutil/rational.h>
}

namespace FoxMediaTerm
{
    namespace
    {
        constexpr int k_MicrosecondsPerSecond = 1'000'000;

        [[nodiscard]] std::string PathForFfmpeg(const std::filesystem::path& path);
        [[nodiscard]] std::string FfmpegError(std::string_view operation, int errorCode);
    } // namespace

    std::expected<VideoDecoder, std::string> VideoDecoder::Open(
        const std::filesystem::path& path,
        VideoPlaybackTelemetry* telemetry
    )
    {
        av_log_set_level(AV_LOG_QUIET);
        if (!HasSupportedExtension(path))
        {
            return std::unexpected("Unsupported video file extension: " + PathForFfmpeg(path));
        }

        auto implementation = std::make_unique<Implementation>();
        implementation->telemetry = telemetry;
        auto decoder = implementation->OpenInput(path);
        if (!decoder)
        {
            return std::unexpected(decoder.error());
        }

        auto opened = implementation->OpenCodec(*decoder);
        if (!opened)
        {
            return std::unexpected(opened.error());
        }

        return VideoDecoder(std::move(implementation));
    }

    std::expected<const AVCodec*, std::string> VideoDecoder::Implementation::OpenInput(
        const std::filesystem::path& path
    )
    {
        const std::string pathText = PathForFfmpeg(path);

        int result = avformat_open_input(&formatContext, pathText.c_str(), nullptr, nullptr);
        if (result < 0)
        {
            return std::unexpected(FfmpegError("Opening video '" + pathText + "'", result));
        }

        result = avformat_find_stream_info(formatContext, nullptr);
        if (result < 0)
        {
            return std::unexpected(FfmpegError("Reading video stream information", result));
        }

        const AVCodec* decoder = nullptr;
        for (unsigned int index = 0; index < formatContext->nb_streams; ++index)
        {
            AVStream* stream = formatContext->streams[index];
            if (stream != nullptr && stream->codecpar != nullptr && stream->codecpar->codec_type == AVMEDIA_TYPE_VIDEO
                && (stream->disposition & AV_DISPOSITION_ATTACHED_PIC) == 0)
            {
                decoder = avcodec_find_decoder(stream->codecpar->codec_id);
                if (decoder != nullptr)
                {
                    videoStreamIndex = static_cast<int>(index);
                    videoStream = stream;
                    break;
                }
            }
        }
        if (videoStream == nullptr)
        {
            return std::unexpected("The media file does not contain a decodable regular video stream");
        }

        return decoder;
    }

    std::expected<void, std::string> VideoDecoder::Implementation::OpenCodec(const AVCodec* decoder)
    {
        codecContext = avcodec_alloc_context3(decoder);
        if (codecContext == nullptr)
        {
            return std::unexpected("Unable to allocate the FFmpeg video decoder context");
        }

        int result = avcodec_parameters_to_context(codecContext, videoStream->codecpar);
        if (result < 0)
        {
            return std::unexpected(FfmpegError("Configuring the video decoder", result));
        }

        result = avcodec_open2(codecContext, decoder, nullptr);
        if (result < 0)
        {
            return std::unexpected(FfmpegError("Opening the video decoder", result));
        }

        decodedFrame = av_frame_alloc();
        packet = av_packet_alloc();
        if (decodedFrame == nullptr || packet == nullptr)
        {
            return std::unexpected("Unable to allocate FFmpeg frame buffers");
        }

        const AVRational frameRate = av_guess_frame_rate(formatContext, videoStream, nullptr);
        if (frameRate.num > 0 && frameRate.den > 0)
        {
            const std::int64_t duration =
                av_rescale_q(1, av_inv_q(frameRate), AVRational { 1, k_MicrosecondsPerSecond });
            if (duration > 0)
            {
                frameDuration = std::chrono::microseconds(duration);
            }
        }

        if (telemetry != nullptr)
        {
            const double framesPerSecond = frameRate.num > 0 && frameRate.den > 0
                                               ? av_q2d(frameRate)
                                               : static_cast<double>(k_MicrosecondsPerSecond) / frameDuration.count();
            telemetry->RecordVideoRate(framesPerSecond, frameDuration);
        }

        return { };
    }

    namespace
    {
        [[nodiscard]] std::string PathForFfmpeg(const std::filesystem::path& path)
        {
            const std::u8string text = path.generic_u8string();
            return std::string(reinterpret_cast<const char*>(text.data()), text.size());
        }

        [[nodiscard]] std::string FfmpegError(std::string_view operation, int errorCode)
        {
            std::array<char, AV_ERROR_MAX_STRING_SIZE> errorText { };
            if (av_strerror(errorCode, errorText.data(), errorText.size()) < 0)
            {
                return std::string(operation) + " failed with FFmpeg error " + std::to_string(errorCode);
            }

            return std::string(operation) + " failed: " + errorText.data();
        }
    } // namespace
} // namespace FoxMediaTerm
