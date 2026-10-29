#include "AudioDecoder.h"

#include <utility>

extern "C"
{
#include <libavutil/frame.h>
#include <libavutil/log.h>
}

#include "AudioDecoderImplementation.h"

namespace FoxMediaTerm
{
    std::size_t AudioPcmChunk::FrameCount() const noexcept
    {
        return samples.size() / AudioDecoder::k_ChannelCount;
    }

    AudioDecoder::AudioDecoder(std::unique_ptr<Implementation> implementation)
        : m_Implementation(std::move(implementation))
    {
    }

    AudioDecoder::~AudioDecoder() = default;
    AudioDecoder::AudioDecoder(AudioDecoder&&) noexcept = default;
    AudioDecoder& AudioDecoder::operator=(AudioDecoder&&) noexcept = default;

    std::expected<std::optional<AudioDecoder>, std::string> AudioDecoder::Open(const std::filesystem::path& path)
    {
        av_log_set_level(AV_LOG_QUIET);
        auto implementation = std::make_unique<Implementation>();

        auto inputOpened = implementation->OpenInput(path);
        if (!inputOpened)
        {
            return std::unexpected(inputOpened.error());
        }

        auto decoder = implementation->SelectAudioStream();
        if (!decoder)
        {
            return std::unexpected(decoder.error());
        }

        if (*decoder == nullptr)
        {
            return std::optional<AudioDecoder> { };
        }

        auto codecOpened = implementation->OpenCodec(*decoder);
        if (!codecOpened)
        {
            return std::unexpected(codecOpened.error());
        }

        return std::optional<AudioDecoder>(AudioDecoder(std::move(implementation)));
    }

    std::expected<std::optional<AudioPcmChunk>, std::string> AudioDecoder::ReadChunk()
    {
        while (true)
        {
            auto received = m_Implementation->ReceiveDecodedFrame();
            if (!received)
            {
                return std::unexpected(received.error());
            }

            if (!*received)
            {
                return m_Implementation->DrainResampler();
            }

            auto chunk = m_Implementation->ConvertFrame();
            av_frame_unref(m_Implementation->decodedFrame);
            if (!chunk)
            {
                return std::unexpected(chunk.error());
            }

            if (*chunk)
            {
                return chunk;
            }
        }
    }

    std::optional<std::chrono::microseconds> AudioDecoder::StreamStartTimestamp() const noexcept
    {
        return m_Implementation->streamStartTimestamp;
    }
} // namespace FoxMediaTerm
