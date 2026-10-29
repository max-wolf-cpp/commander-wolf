#pragma once

#include <chrono>
#include <expected>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace FoxMediaTerm
{
    struct AudioPcmChunk
    {
        std::chrono::microseconds timestamp { 0 };
        std::vector<float> samples;

        [[nodiscard]] std::size_t FrameCount() const noexcept;
    };

    class AudioDecoder
    {
      public:
        static constexpr int k_SampleRate = 48'000;
        static constexpr int k_ChannelCount = 2;

        ~AudioDecoder();

        AudioDecoder(AudioDecoder&&) noexcept;
        AudioDecoder& operator=(AudioDecoder&&) noexcept;
        AudioDecoder(const AudioDecoder&) = delete;
        AudioDecoder& operator=(const AudioDecoder&) = delete;

        [[nodiscard]] static std::expected<std::optional<AudioDecoder>, std::string> Open(
            const std::filesystem::path& path
        );
        [[nodiscard]] std::expected<std::optional<AudioPcmChunk>, std::string> ReadChunk();
        [[nodiscard]] std::optional<std::chrono::microseconds> StreamStartTimestamp() const noexcept;

      private:
        struct Implementation;

        explicit AudioDecoder(std::unique_ptr<Implementation> implementation);

        std::unique_ptr<Implementation> m_Implementation;
    };
} // namespace FoxMediaTerm
