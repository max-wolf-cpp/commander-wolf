#pragma once

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <expected>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>

#include "AudioVisualization.h"

namespace FoxMediaTerm
{
    struct AudioPlaybackStatistics
    {
        std::uint64_t decodedPcmFrames = 0;
        std::uint64_t queuedPcmFrames = 0;
        std::uint64_t playedPcmFrames = 0;
        std::uint64_t underrunFrames = 0;
        std::size_t bufferCapacityFrames = 0;
        bool decoderFinished = false;
    };

    class AudioPlayback
    {
      public:
        using Clock = std::chrono::steady_clock;

        static constexpr int k_SampleRate = 48'000;
        static constexpr int k_ChannelCount = 2;

        ~AudioPlayback();

        AudioPlayback(const AudioPlayback&) = delete;
        AudioPlayback& operator=(const AudioPlayback&) = delete;
        AudioPlayback(AudioPlayback&&) = delete;
        AudioPlayback& operator=(AudioPlayback&&) = delete;

        [[nodiscard]] static std::expected<std::unique_ptr<AudioPlayback>, std::string> Open(
            const std::filesystem::path& path
        );
        void SetMuted(bool muted) noexcept;

        [[nodiscard]] static std::chrono::microseconds MediaTimeForPlayedFrames(
            std::chrono::microseconds mediaStart,
            std::uint64_t playedFrames
        ) noexcept;

        [[nodiscard]] std::chrono::microseconds CurrentMediaTime() const noexcept;
        [[nodiscard]] std::chrono::microseconds MediaStart() const noexcept;
        [[nodiscard]] Clock::time_point PlaybackStart() const noexcept;
        [[nodiscard]] std::optional<std::string> Error() const;
        [[nodiscard]] AudioPlaybackStatistics Statistics() const noexcept;
        [[nodiscard]] bool IsFinished() const noexcept;
        [[nodiscard]] AudioVisualization::Levels VisualizationLevels() const noexcept;
        [[nodiscard]] bool IsMuted() const noexcept;

      private:
        struct Implementation;

        explicit AudioPlayback(std::unique_ptr<Implementation> implementation);

        std::unique_ptr<Implementation> m_Implementation;
    };
} // namespace FoxMediaTerm
