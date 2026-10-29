#include "AudioVisualization.h"

#include <algorithm>
#include <cmath>

namespace FoxMediaTerm
{
    AudioVisualization::Levels AudioVisualization::CalculateLevels(
        std::span<const float> interleavedSamples,
        std::size_t channelCount
    ) noexcept
    {
        Levels levels { };
        if (channelCount == 0)
        {
            return levels;
        }

        const std::size_t frameCount = interleavedSamples.size() / channelCount;
        if (frameCount == 0)
        {
            return levels;
        }

        for (std::size_t levelIndex = 0; levelIndex < levels.size(); ++levelIndex)
        {
            const std::size_t firstFrame = levelIndex * frameCount / levels.size();
            const std::size_t lastFrame = std::max(firstFrame + 1, (levelIndex + 1) * frameCount / levels.size());

            float peak = 0.0F;
            for (std::size_t frame = firstFrame; frame < std::min(lastFrame, frameCount); ++frame)
            {
                for (std::size_t channel = 0; channel < channelCount; ++channel)
                {
                    peak = std::max(peak, std::abs(interleavedSamples[frame * channelCount + channel]));
                }
            }

            levels[levelIndex] = std::clamp(peak, 0.0F, 1.0F);
        }

        return levels;
    }

    void AudioVisualization::Update(std::span<const float> interleavedSamples, std::size_t channelCount) noexcept
    {
        const Levels levels = CalculateLevels(interleavedSamples, channelCount);

        m_Generation.fetch_add(1, std::memory_order_acq_rel);
        for (std::size_t index = 0; index < levels.size(); ++index)
        {
            m_Levels[index].store(levels[index], std::memory_order_relaxed);
        }
        m_Generation.fetch_add(1, std::memory_order_release);
    }

    AudioVisualization::Levels AudioVisualization::Snapshot() const noexcept
    {
        Levels levels { };
        std::uint64_t before = 0;
        std::uint64_t after = 0;

        do
        {
            before = m_Generation.load(std::memory_order_acquire);
            if ((before & 1U) != 0U)
            {
                continue;
            }

            for (std::size_t index = 0; index < levels.size(); ++index)
            {
                levels[index] = m_Levels[index].load(std::memory_order_relaxed);
            }

            after = m_Generation.load(std::memory_order_acquire);
        }
        while (before != after || (after & 1U) != 0U);

        return levels;
    }
} // namespace FoxMediaTerm
