#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <span>

namespace FoxMediaTerm
{
    class AudioVisualization
    {
      public:
        static constexpr std::size_t k_LevelCount = 64;
        using Levels = std::array<float, k_LevelCount>;

        [[nodiscard]] static Levels CalculateLevels(
            std::span<const float> interleavedSamples,
            std::size_t channelCount
        ) noexcept;

        // A single writer publishes levels; readers may take snapshots concurrently.
        void Update(std::span<const float> interleavedSamples, std::size_t channelCount) noexcept;

        [[nodiscard]] Levels Snapshot() const noexcept;

      private:
        std::array<std::atomic<float>, k_LevelCount> m_Levels { };
        // Odd generations mark an update in progress; even generations mark complete levels.
        std::atomic<std::uint64_t> m_Generation { 0 };
    };
} // namespace FoxMediaTerm
