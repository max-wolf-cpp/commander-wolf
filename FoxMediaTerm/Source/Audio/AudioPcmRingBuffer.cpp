#include "AudioPcmRingBuffer.h"

#include <algorithm>
#include <cassert>
#include <cstring>

namespace FoxMediaTerm
{
    AudioPcmRingBuffer::AudioPcmRingBuffer(std::size_t capacityFrames)
        : m_Samples(std::max<std::size_t>(capacityFrames, 1) * k_ChannelCount)
        , m_CapacityFrames(std::max<std::size_t>(capacityFrames, 1))
    {
    }

    std::size_t AudioPcmRingBuffer::Write(std::span<const float> interleavedSamples) noexcept
    {
        assert(interleavedSamples.size() % k_ChannelCount == 0);

        const std::uint64_t writeFrame = m_WriteFrame.load(std::memory_order_relaxed);
        const std::uint64_t readFrame = m_ReadFrame.load(std::memory_order_acquire);
        const std::size_t requestedFrames = interleavedSamples.size() / k_ChannelCount;
        const std::size_t occupiedFrames = static_cast<std::size_t>(writeFrame - readFrame);
        const std::size_t writableFrames = std::min(requestedFrames, m_CapacityFrames - occupiedFrames);

        CopyInto(writeFrame, interleavedSamples.first(writableFrames * k_ChannelCount));
        m_WriteFrame.store(writeFrame + writableFrames, std::memory_order_release);

        return writableFrames;
    }

    std::size_t AudioPcmRingBuffer::Read(std::span<float> interleavedSamples) noexcept
    {
        assert(interleavedSamples.size() % k_ChannelCount == 0);

        const std::uint64_t readFrame = m_ReadFrame.load(std::memory_order_relaxed);
        const std::uint64_t writeFrame = m_WriteFrame.load(std::memory_order_acquire);
        const std::size_t requestedFrames = interleavedSamples.size() / k_ChannelCount;
        const std::size_t readableFrames = std::min(requestedFrames, static_cast<std::size_t>(writeFrame - readFrame));

        CopyOut(readFrame, interleavedSamples.first(readableFrames * k_ChannelCount));
        m_ReadFrame.store(readFrame + readableFrames, std::memory_order_release);

        return readableFrames;
    }

    std::size_t AudioPcmRingBuffer::CapacityFrames() const noexcept
    {
        return m_CapacityFrames;
    }

    std::size_t AudioPcmRingBuffer::AvailableReadFrames() const noexcept
    {
        const std::uint64_t writeFrame = m_WriteFrame.load(std::memory_order_acquire);
        const std::uint64_t readFrame = m_ReadFrame.load(std::memory_order_acquire);
        return static_cast<std::size_t>(writeFrame - readFrame);
    }

    std::size_t AudioPcmRingBuffer::AvailableWriteFrames() const noexcept
    {
        return m_CapacityFrames - AvailableReadFrames();
    }

    void AudioPcmRingBuffer::CopyInto(std::uint64_t frameOffset, std::span<const float> samples) noexcept
    {
        if (samples.empty())
        {
            return;
        }

        const std::size_t frameCount = samples.size() / k_ChannelCount;
        const std::size_t startFrame = static_cast<std::size_t>(frameOffset % m_CapacityFrames);
        const std::size_t firstFrames = std::min(frameCount, m_CapacityFrames - startFrame);
        const std::size_t firstSamples = firstFrames * k_ChannelCount;

        std::memcpy(m_Samples.data() + startFrame * k_ChannelCount, samples.data(), firstSamples * sizeof(float));
        if (firstFrames < frameCount)
        {
            std::memcpy(
                m_Samples.data(),
                samples.data() + firstSamples,
                (samples.size() - firstSamples) * sizeof(float)
            );
        }
    }

    void AudioPcmRingBuffer::CopyOut(std::uint64_t frameOffset, std::span<float> samples) const noexcept
    {
        if (samples.empty())
        {
            return;
        }

        const std::size_t frameCount = samples.size() / k_ChannelCount;
        const std::size_t startFrame = static_cast<std::size_t>(frameOffset % m_CapacityFrames);
        const std::size_t firstFrames = std::min(frameCount, m_CapacityFrames - startFrame);
        const std::size_t firstSamples = firstFrames * k_ChannelCount;

        std::memcpy(samples.data(), m_Samples.data() + startFrame * k_ChannelCount, firstSamples * sizeof(float));
        if (firstFrames < frameCount)
        {
            std::memcpy(
                samples.data() + firstSamples,
                m_Samples.data(),
                (samples.size() - firstSamples) * sizeof(float)
            );
        }
    }
} // namespace FoxMediaTerm
