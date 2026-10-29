#include "VideoDecoder.h"

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "MediaProbe.h"
#include "VideoDecoderImplementation.h"
#include "VideoPlaybackTelemetry.h"

extern "C"
{
#include <libavutil/error.h>
#include <libavutil/pixdesc.h>
#include <libavutil/rational.h>
}

namespace FoxMediaTerm
{
    namespace
    {
        constexpr int k_MicrosecondsPerSecond = 1'000'000;

        [[nodiscard]] std::string FfmpegError(std::string_view operation, int errorCode)
        {
            std::array<char, AV_ERROR_MAX_STRING_SIZE> errorText { };
            if (av_strerror(errorCode, errorText.data(), errorText.size()) < 0)
            {
                return std::string(operation) + " failed with FFmpeg error " + std::to_string(errorCode);
            }

            return std::string(operation) + " failed: " + errorText.data();
        }

        struct FrameSize
        {
            int width = 0;
            int height = 0;
        };

        [[nodiscard]] bool SourceHasAlpha(AVPixelFormat pixelFormat)
        {
            const AVPixFmtDescriptor* descriptor = av_pix_fmt_desc_get(pixelFormat);

            return descriptor != nullptr && (descriptor->flags & AV_PIX_FMT_FLAG_ALPHA) != 0;
        }

        [[nodiscard]] FrameSize CalculateFrameSize(
            int sourceWidth,
            int sourceHeight,
            AVRational sampleAspectRatio,
            int maximumWidth,
            int maximumHeight
        )
        {
            long double aspectRatio = static_cast<long double>(sourceWidth) / sourceHeight;
            if (sampleAspectRatio.num > 0 && sampleAspectRatio.den > 0)
            {
                aspectRatio *= static_cast<long double>(sampleAspectRatio.num) / sampleAspectRatio.den;
            }

            int width = maximumWidth;
            int height = static_cast<int>(std::llround(static_cast<long double>(width) / aspectRatio));
            if (height > maximumHeight)
            {
                height = maximumHeight;
                width = static_cast<int>(std::llround(static_cast<long double>(height) * aspectRatio));
            }

            return {
                std::clamp(width, 1, maximumWidth),
                std::clamp(height, 1, maximumHeight),
            };
        }
    } // namespace

    VideoDecoder::VideoDecoder(std::unique_ptr<Implementation> implementation)
        : m_Implementation(std::move(implementation))
    {
    }

    VideoDecoder::~VideoDecoder() = default;
    VideoDecoder::VideoDecoder(VideoDecoder&& other) noexcept = default;
    VideoDecoder& VideoDecoder::operator=(VideoDecoder&& other) noexcept = default;

    bool VideoDecoder::HasSupportedExtension(const std::filesystem::path& path)
    {
        return MediaProbe::HasSupportedExtension(path);
    }

    std::expected<VideoMediaInfo, std::string> VideoDecoder::Probe(const std::filesystem::path& path)
    {
        auto media = MediaProbe::Probe(path);
        if (!media)
        {
            return std::unexpected(media.error());
        }

        if (media->kind != EMediaStreamKind::Video)
        {
            return std::unexpected("The media file does not contain a regular video stream");
        }

        return VideoMediaInfo {
            .width = media->width,
            .height = media->height,
            .duration = media->duration,
            .hasAudio = media->hasAudio,
        };
    }

    std::expected<std::optional<VideoFrame>, std::string> VideoDecoder::ReadFrame(int maximumWidth, int maximumHeight)
    {
        if (maximumWidth <= 0 || maximumHeight <= 0)
        {
            return std::unexpected("The terminal video area must be positive");
        }

        auto received = m_Implementation->ReceiveDecodedFrame();
        if (!received)
        {
            return std::unexpected(received.error());
        }

        if (!*received)
        {
            return std::optional<VideoFrame> { };
        }

        auto image = m_Implementation->ConvertFrame(maximumWidth, maximumHeight);
        if (!image)
        {
            return std::unexpected(image.error());
        }

        return std::optional<VideoFrame>(VideoFrame {
            std::move(*image),
            m_Implementation->ReadTimestamp(),
            m_Implementation->frameDuration,
            m_Implementation->nextFrameIndex++,
        });
    }

    VideoDecoder::Implementation::~Implementation()
    {
        sws_freeContext(scaleContext);
        av_packet_free(&packet);
        av_frame_free(&decodedFrame);
        avcodec_free_context(&codecContext);
        avformat_close_input(&formatContext);
    }

    std::expected<bool, std::string> VideoDecoder::Implementation::ReceiveDecodedFrame()
    {
        const auto frameStart = std::chrono::steady_clock::now();
        std::chrono::steady_clock::duration demuxDuration { };
        std::chrono::steady_clock::duration codecDuration { };
        while (true)
        {
            const auto receiveStart = std::chrono::steady_clock::now();
            const int receiveResult = avcodec_receive_frame(codecContext, decodedFrame);
            codecDuration += std::chrono::steady_clock::now() - receiveStart;
            if (receiveResult == 0)
            {
                if (telemetry != nullptr)
                {
                    const char* pixelFormat = av_get_pix_fmt_name(static_cast<AVPixelFormat>(decodedFrame->format));
                    telemetry->RecordSourceGeometry(
                        decodedFrame->width,
                        decodedFrame->height,
                        pixelFormat == nullptr ? "unknown" : pixelFormat
                    );
                    telemetry
                        ->RecordDecode(std::chrono::steady_clock::now() - frameStart, demuxDuration, codecDuration);
                }

                return true;
            }

            if (receiveResult == AVERROR_EOF)
            {
                return false;
            }

            if (receiveResult != AVERROR(EAGAIN))
            {
                return std::unexpected(FfmpegError("Receiving a decoded video frame", receiveResult));
            }

            if (drainPacketSent)
            {
                return std::unexpected("FFmpeg requested another packet after the decoder was drained");
            }

            int readResult = 0;
            while (true)
            {
                const auto demuxStart = std::chrono::steady_clock::now();
                readResult = av_read_frame(formatContext, packet);
                demuxDuration += std::chrono::steady_clock::now() - demuxStart;
                if (readResult < 0 || packet->stream_index == videoStreamIndex)
                {
                    break;
                }

                av_packet_unref(packet);
            }

            if (readResult == AVERROR_EOF)
            {
                drainPacketSent = true;
                const auto sendStart = std::chrono::steady_clock::now();
                const int sendResult = avcodec_send_packet(codecContext, nullptr);
                codecDuration += std::chrono::steady_clock::now() - sendStart;
                if (sendResult < 0 && sendResult != AVERROR_EOF)
                {
                    return std::unexpected(FfmpegError("Draining the video decoder", sendResult));
                }

                continue;
            }

            if (readResult < 0)
            {
                return std::unexpected(FfmpegError("Reading a video packet", readResult));
            }

            const auto sendStart = std::chrono::steady_clock::now();
            const int sendResult = avcodec_send_packet(codecContext, packet);
            codecDuration += std::chrono::steady_clock::now() - sendStart;
            av_packet_unref(packet);
            if (sendResult < 0)
            {
                return std::unexpected(FfmpegError("Sending a packet to the video decoder", sendResult));
            }
        }
    }

    std::chrono::microseconds VideoDecoder::Implementation::ReadTimestamp()
    {
        std::chrono::microseconds timestamp = fallbackTimestamp;
        if (decodedFrame->best_effort_timestamp != AV_NOPTS_VALUE)
        {
            timestamp = std::chrono::microseconds(av_rescale_q(
                decodedFrame->best_effort_timestamp,
                videoStream->time_base,
                AVRational { 1, k_MicrosecondsPerSecond }
            ));
        }

        if (lastTimestamp && timestamp < *lastTimestamp)
        {
            timestamp = *lastTimestamp;
        }

        lastTimestamp = timestamp;
        fallbackTimestamp = timestamp + frameDuration;

        return timestamp;
    }

    std::expected<RasterImage, std::string> VideoDecoder::Implementation::ConvertFrame(
        int maximumWidth,
        int maximumHeight
    )
    {
        const AVRational sampleAspectRatio = av_guess_sample_aspect_ratio(formatContext, videoStream, decodedFrame);
        const FrameSize outputSize = CalculateFrameSize(
            decodedFrame->width,
            decodedFrame->height,
            sampleAspectRatio,
            maximumWidth,
            maximumHeight
        );

        const std::size_t width = static_cast<std::size_t>(outputSize.width);
        const std::size_t height = static_cast<std::size_t>(outputSize.height);
        const AVPixelFormat sourcePixelFormat = static_cast<AVPixelFormat>(decodedFrame->format);
        const EImagePixelFormat outputPixelFormat =
            SourceHasAlpha(sourcePixelFormat) ? EImagePixelFormat::Rgba32 : EImagePixelFormat::Rgb24;
        const std::size_t channels = ChannelCount(outputPixelFormat);
        if (width > std::numeric_limits<std::size_t>::max() / height / channels)
        {
            return std::unexpected("The scaled video frame is too large");
        }

        const AVPixelFormat ffmpegOutputPixelFormat = HasAlpha(outputPixelFormat) ? AV_PIX_FMT_RGBA : AV_PIX_FMT_RGB24;

        scaleContext = sws_getCachedContext(
            scaleContext,
            decodedFrame->width,
            decodedFrame->height,
            sourcePixelFormat,
            outputSize.width,
            outputSize.height,
            ffmpegOutputPixelFormat,
            SWS_BILINEAR,
            nullptr,
            nullptr,
            nullptr
        );
        if (scaleContext == nullptr)
        {
            return std::unexpected("Unable to create the FFmpeg video scaling context");
        }

        // swscale SIMD paths may touch the documented FFmpeg padding beyond the final row.
        // Keep it during conversion, then expose the exact tightly packed pixel buffer.
        RasterImage image {
            outputSize.width,
            outputSize.height,
            std::vector<std::uint8_t>(width * height * channels + AV_INPUT_BUFFER_PADDING_SIZE),
            outputPixelFormat,
        };
        std::uint8_t* destinationData[] = { image.pixels.data(), nullptr, nullptr, nullptr };
        const int destinationLineSizes[] = { outputSize.width * static_cast<int>(channels), 0, 0, 0 };
        const auto scaleStart = std::chrono::steady_clock::now();
        const int scaledRows = sws_scale(
            scaleContext,
            decodedFrame->data,
            decodedFrame->linesize,
            0,
            decodedFrame->height,
            destinationData,
            destinationLineSizes
        );
        const auto scaleEnd = std::chrono::steady_clock::now();
        if (telemetry != nullptr)
        {
            telemetry->RecordScale(scaleEnd - scaleStart);
        }

        if (scaledRows != outputSize.height)
        {
            return std::unexpected("FFmpeg did not produce the complete scaled video frame");
        }

        image.pixels.resize(width * height * channels);
        if (telemetry != nullptr)
        {
            telemetry->RecordScaledGeometry(
                image.width,
                image.height,
                static_cast<std::uint64_t>(image.pixels.size()),
                ImagePixelFormatName(image.pixelFormat)
            );
        }

        return image;
    }
} // namespace FoxMediaTerm
