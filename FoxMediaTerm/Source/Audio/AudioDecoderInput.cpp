#include <array>

extern "C"
{
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/error.h>
#include <libswresample/swresample.h>
}

#include "AudioDecoderImplementation.h"

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] std::string PathForFfmpeg(const std::filesystem::path& path)
        {
            const std::u8string text = path.generic_u8string();
            return std::string(reinterpret_cast<const char*>(text.data()), text.size());
        }
    } // namespace

    AudioDecoder::Implementation::~Implementation()
    {
        av_channel_layout_uninit(&inputLayout);
        swr_free(&resampleContext);
        av_packet_free(&packet);
        av_frame_free(&decodedFrame);
        avcodec_free_context(&codecContext);
        avformat_close_input(&formatContext);
    }

    std::expected<void, std::string> AudioDecoder::Implementation::OpenInput(const std::filesystem::path& path)
    {
        const std::string pathText = PathForFfmpeg(path);
        const int openResult = avformat_open_input(&formatContext, pathText.c_str(), nullptr, nullptr);
        if (openResult < 0)
        {
            return std::unexpected(FfmpegError("Opening audio from '" + pathText + "'", openResult));
        }

        const int informationResult = avformat_find_stream_info(formatContext, nullptr);
        if (informationResult < 0)
        {
            return std::unexpected(FfmpegError("Reading audio stream information", informationResult));
        }

        return { };
    }

    std::expected<const AVCodec*, std::string> AudioDecoder::Implementation::SelectAudioStream()
    {
        const AVCodec* decoder = nullptr;
        const int streamIndex = av_find_best_stream(formatContext, AVMEDIA_TYPE_AUDIO, -1, -1, &decoder, 0);
        if (streamIndex == AVERROR_STREAM_NOT_FOUND)
        {
            return nullptr;
        }

        if (streamIndex < 0)
        {
            return std::unexpected(FfmpegError("Finding an audio stream", streamIndex));
        }

        audioStreamIndex = streamIndex;
        audioStream = formatContext->streams[streamIndex];
        if (audioStream->start_time != AV_NOPTS_VALUE)
        {
            streamStartTimestamp = RescaleTimestamp(audioStream->start_time, audioStream->time_base);
            fallbackTimestamp = *streamStartTimestamp;
        }

        return decoder;
    }

    std::expected<void, std::string> AudioDecoder::Implementation::OpenCodec(const AVCodec* decoder)
    {
        codecContext = avcodec_alloc_context3(decoder);
        if (codecContext == nullptr)
        {
            return std::unexpected("Unable to allocate the FFmpeg audio decoder context");
        }

        const int parametersResult = avcodec_parameters_to_context(codecContext, audioStream->codecpar);
        if (parametersResult < 0)
        {
            return std::unexpected(FfmpegError("Configuring the audio decoder", parametersResult));
        }

        const int openResult = avcodec_open2(codecContext, decoder, nullptr);
        if (openResult < 0)
        {
            return std::unexpected(FfmpegError("Opening the audio decoder", openResult));
        }

        decodedFrame = av_frame_alloc();
        packet = av_packet_alloc();
        if (decodedFrame == nullptr || packet == nullptr)
        {
            return std::unexpected("Unable to allocate FFmpeg audio buffers");
        }

        return { };
    }

    std::expected<bool, std::string> AudioDecoder::Implementation::ReceiveDecodedFrame()
    {
        while (true)
        {
            const int receiveResult = avcodec_receive_frame(codecContext, decodedFrame);
            if (receiveResult == 0)
            {
                return true;
            }

            if (receiveResult == AVERROR_EOF)
            {
                return false;
            }

            if (receiveResult != AVERROR(EAGAIN))
            {
                return std::unexpected(FfmpegError("Receiving a decoded audio frame", receiveResult));
            }

            if (drainPacketSent)
            {
                return std::unexpected("FFmpeg requested another audio packet after decoder drain");
            }

            const int readResult = ReadAudioPacket();
            if (readResult == AVERROR_EOF)
            {
                drainPacketSent = true;
                const int sendResult = avcodec_send_packet(codecContext, nullptr);
                if (sendResult < 0 && sendResult != AVERROR_EOF)
                {
                    return std::unexpected(FfmpegError("Draining the audio decoder", sendResult));
                }

                continue;
            }

            if (readResult < 0)
            {
                return std::unexpected(FfmpegError("Reading an audio packet", readResult));
            }

            const int sendResult = avcodec_send_packet(codecContext, packet);
            av_packet_unref(packet);
            if (sendResult < 0)
            {
                return std::unexpected(FfmpegError("Sending a packet to the audio decoder", sendResult));
            }
        }
    }

    int AudioDecoder::Implementation::ReadAudioPacket()
    {
        while (true)
        {
            const int readResult = av_read_frame(formatContext, packet);
            if (readResult < 0 || packet->stream_index == audioStreamIndex)
            {
                return readResult;
            }

            av_packet_unref(packet);
        }
    }

    std::string AudioDecoder::Implementation::FfmpegError(std::string_view operation, int errorCode)
    {
        std::array<char, AV_ERROR_MAX_STRING_SIZE> errorText { };
        if (av_strerror(errorCode, errorText.data(), errorText.size()) < 0)
        {
            return std::string(operation) + " failed with FFmpeg error " + std::to_string(errorCode);
        }

        return std::string(operation) + " failed: " + errorText.data();
    }
} // namespace FoxMediaTerm
