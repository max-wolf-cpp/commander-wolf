#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <span>
#include <vector>

namespace FoxMediaTerm
{
    // One producer writes PCM frames while one consumer reads them.
    class AudioPcmRingBuffer
    {
      public:
        static constexpr std::size_t k_ChannelCount = 2;

        explicit AudioPcmRingBuffer(std::size_t capacityFrames);

        [[nodiscard]] std::size_t Write(std::span<const float> interleavedSamples) noexcept;
        [[nodiscard]] std::size_t Read(std::span<float> interleavedSamples) noexcept;

        [[nodiscard]] std::size_t CapacityFrames() const noexcept;
        [[nodiscard]] std::size_t AvailableReadFrames() const noexcept;
        [[nodiscard]] std::size_t AvailableWriteFrames() const noexcept;

      private:
        void CopyInto(std::uint64_t frameOffset, std::span<const float> samples) noexcept;
        void CopyOut(std::uint64_t frameOffset, std::span<float> samples) const noexcept;

        std::vector<float> m_Samples;
        std::size_t m_CapacityFrames = 0;
        alignas(64) std::atomic<std::uint64_t> m_ReadFrame { 0 };
        alignas(64) std::atomic<std::uint64_t> m_WriteFrame { 0 };
    };
} // namespace FoxMediaTerm
