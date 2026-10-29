#include <algorithm>
#include <array>
#include <utility>

#include "AudioPlaybackImplementation.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::size_t k_PrimeMilliseconds = 250;
        constexpr std::chrono::milliseconds k_BufferRetryDelay { 1 };
        constexpr std::int64_t k_MicrosecondsPerSecond = 1'000'000;

        [[nodiscard]] std::int64_t TimestampToFrame(std::chrono::microseconds timestamp) noexcept
        {
            return timestamp.count() * AudioPlayback::k_SampleRate / k_MicrosecondsPerSecond;
        }
    } // namespace

    std::expected<void, std::string> AudioPlayback::Implementation::Prime(const AudioPcmChunk& firstChunk)
    {
        if (!QueueChunk(firstChunk))
        {
            return std::unexpected("Unable to prime the audio ring buffer");
        }

        const std::size_t primeFrames =
            static_cast<std::size_t>(AudioPlayback::k_SampleRate) * k_PrimeMilliseconds / 1'000;
        while (ring.AvailableReadFrames() < primeFrames)
        {
            auto chunk = decoder.ReadChunk();
            if (!chunk)
            {
                return std::unexpected(chunk.error());
            }

            if (!*chunk)
            {
                decoderFinished.store(true, std::memory_order_release);
                break;
            }

            if (!QueueChunk(**chunk))
            {
                return std::unexpected("Unable to prime the audio ring buffer");
            }
        }

        return { };
    }

    void AudioPlayback::Implementation::Run(std::stop_token stopToken)
    {
        while (!stopToken.stop_requested())
        {
            auto chunk = decoder.ReadChunk();
            if (!chunk)
            {
                SetError(chunk.error());
                return;
            }

            if (!*chunk)
            {
                decoderFinished.store(true, std::memory_order_release);
                return;
            }

            if (!QueueChunk(**chunk, stopToken))
            {
                return;
            }
        }
    }

    bool AudioPlayback::Implementation::QueueChunk(const AudioPcmChunk& chunk, std::stop_token stopToken)
    {
        decodedPcmFrames.fetch_add(chunk.FrameCount(), std::memory_order_relaxed);
        timelineFrame = std::max(timelineFrame, playedFrames.load(std::memory_order_acquire));

        const std::int64_t signedTarget = TimestampToFrame(chunk.timestamp - mediaStart);
        if (signedTarget > 0 && static_cast<std::uint64_t>(signedTarget) > timelineFrame)
        {
            const std::uint64_t silenceFrames = static_cast<std::uint64_t>(signedTarget) - timelineFrame;
            if (!WriteSilence(silenceFrames, stopToken))
            {
                return false;
            }

            timelineFrame += silenceFrames;
            queuedPcmFrames.fetch_add(silenceFrames, std::memory_order_relaxed);
        }

        const std::uint64_t targetFrame = signedTarget > 0 ? static_cast<std::uint64_t>(signedTarget) : 0;
        const std::uint64_t obsoleteFrames = timelineFrame > targetFrame ? timelineFrame - targetFrame : 0;
        const std::size_t skippedFrames =
            static_cast<std::size_t>(std::min<std::uint64_t>(obsoleteFrames, chunk.FrameCount()));
        const std::span<const float> remaining =
            std::span<const float>(chunk.samples).subspan(skippedFrames * AudioPcmRingBuffer::k_ChannelCount);
        if (!WriteFrames(remaining, stopToken))
        {
            return false;
        }

        const std::size_t queuedFrames = remaining.size() / AudioPcmRingBuffer::k_ChannelCount;
        timelineFrame += queuedFrames;
        queuedPcmFrames.fetch_add(queuedFrames, std::memory_order_relaxed);

        return true;
    }

    bool AudioPlayback::Implementation::WriteSilence(std::uint64_t frameCount, std::stop_token stopToken)
    {
        constexpr std::size_t k_SilenceBlockFrames = 2'048;
        const std::array<float, k_SilenceBlockFrames * AudioPcmRingBuffer::k_ChannelCount> silence { };

        while (frameCount > 0 && !stopToken.stop_requested())
        {
            const std::size_t blockFrames =
                static_cast<std::size_t>(std::min<std::uint64_t>(frameCount, k_SilenceBlockFrames));
            if (!WriteFrames(
                    std::span<const float>(silence.data(), blockFrames * AudioPcmRingBuffer::k_ChannelCount),
                    stopToken
                ))
            {
                return false;
            }

            frameCount -= blockFrames;
        }

        return frameCount == 0;
    }

    bool AudioPlayback::Implementation::WriteFrames(std::span<const float> samples, std::stop_token stopToken)
    {
        std::size_t writtenFrames = 0;
        const std::size_t totalFrames = samples.size() / AudioPcmRingBuffer::k_ChannelCount;

        while (writtenFrames < totalFrames && !stopToken.stop_requested())
        {
            const std::span<const float> remaining =
                samples.subspan(writtenFrames * AudioPcmRingBuffer::k_ChannelCount);
            const std::size_t written = ring.Write(remaining);
            writtenFrames += written;
            if (written == 0)
            {
                std::this_thread::sleep_for(k_BufferRetryDelay);
            }
        }

        return writtenFrames == totalFrames;
    }

    void AudioPlayback::Implementation::SetError(std::string message)
    {
        std::lock_guard lock(errorMutex);
        if (!error)
        {
            error = std::move(message);
        }
    }
} // namespace FoxMediaTerm
