#include "AudioPlayback.h"

#include <utility>

#include "AudioPlaybackImplementation.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::chrono::microseconds k_MaxInitialTimestampGap { 500'000 };
        constexpr std::int64_t k_MicrosecondsPerSecond = 1'000'000;

        [[nodiscard]] std::chrono::microseconds FrameToTimestamp(std::uint64_t frame) noexcept
        {
            return std::chrono::microseconds(
                static_cast<std::int64_t>(frame * k_MicrosecondsPerSecond / AudioPlayback::k_SampleRate)
            );
        }
    } // namespace

    AudioPlayback::AudioPlayback(std::unique_ptr<Implementation> implementation)
        : m_Implementation(std::move(implementation))
    {
    }

    AudioPlayback::~AudioPlayback() = default;

    std::expected<std::unique_ptr<AudioPlayback>, std::string> AudioPlayback::Open(const std::filesystem::path& path)
    {
        auto openedDecoder = AudioDecoder::Open(path);
        if (!openedDecoder)
        {
            return std::unexpected(openedDecoder.error());
        }

        if (!*openedDecoder)
        {
            return std::unique_ptr<AudioPlayback> { };
        }

        AudioDecoder decoder = std::move(**openedDecoder);
        auto firstChunk = decoder.ReadChunk();
        if (!firstChunk)
        {
            return std::unexpected(firstChunk.error());
        }

        if (!*firstChunk)
        {
            return std::unique_ptr<AudioPlayback> { };
        }

        std::chrono::microseconds mediaStart = (**firstChunk).timestamp;
        if (const auto streamStart = decoder.StreamStartTimestamp();
            streamStart && (**firstChunk).timestamp >= *streamStart)
        {
            const std::chrono::microseconds gap = (**firstChunk).timestamp - *streamStart;
            if (gap <= k_MaxInitialTimestampGap)
            {
                mediaStart = *streamStart;
            }
        }

        auto playback = std::unique_ptr<AudioPlayback>(
            new AudioPlayback(std::make_unique<Implementation>(std::move(decoder), mediaStart))
        );
        Implementation& implementation = *playback->m_Implementation;
        auto primed = implementation.Prime(**firstChunk);
        if (!primed)
        {
            return std::unexpected(primed.error());
        }

        auto started = implementation.StartDevice();
        if (!started)
        {
            return std::unexpected(started.error());
        }

        return playback;
    }

    void AudioPlayback::SetMuted(bool muted) noexcept
    {
        m_Implementation->muted.store(muted, std::memory_order_relaxed);
    }

    std::chrono::microseconds AudioPlayback::MediaTimeForPlayedFrames(
        std::chrono::microseconds mediaStart,
        std::uint64_t playedFrames
    ) noexcept
    {
        return mediaStart + FrameToTimestamp(playedFrames);
    }

    std::chrono::microseconds AudioPlayback::CurrentMediaTime() const noexcept
    {
        return MediaTimeForPlayedFrames(
            m_Implementation->mediaStart,
            m_Implementation->playedFrames.load(std::memory_order_acquire)
        );
    }

    std::chrono::microseconds AudioPlayback::MediaStart() const noexcept
    {
        return m_Implementation->mediaStart;
    }

    AudioPlayback::Clock::time_point AudioPlayback::PlaybackStart() const noexcept
    {
        return m_Implementation->playbackStart;
    }

    std::optional<std::string> AudioPlayback::Error() const
    {
        std::lock_guard lock(m_Implementation->errorMutex);
        return m_Implementation->error;
    }

    AudioPlaybackStatistics AudioPlayback::Statistics() const noexcept
    {
        return {
            .decodedPcmFrames = m_Implementation->decodedPcmFrames.load(std::memory_order_relaxed),
            .queuedPcmFrames = m_Implementation->queuedPcmFrames.load(std::memory_order_relaxed),
            .playedPcmFrames = m_Implementation->playedFrames.load(std::memory_order_relaxed),
            .underrunFrames = m_Implementation->underrunFrames.load(std::memory_order_relaxed),
            .bufferCapacityFrames = m_Implementation->ring.CapacityFrames(),
            .decoderFinished = m_Implementation->decoderFinished.load(std::memory_order_relaxed),
        };
    }

    bool AudioPlayback::IsFinished() const noexcept
    {
        return m_Implementation->decoderFinished.load(std::memory_order_acquire)
               && m_Implementation->ring.AvailableReadFrames() == 0;
    }

    AudioVisualization::Levels AudioPlayback::VisualizationLevels() const noexcept
    {
        return m_Implementation->visualization.Snapshot();
    }

    bool AudioPlayback::IsMuted() const noexcept
    {
        return m_Implementation->muted.load(std::memory_order_relaxed);
    }
} // namespace FoxMediaTerm
