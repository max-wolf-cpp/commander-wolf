#pragma once

#include <chrono>
#include <expected>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>

#include "VideoFrame.h"

namespace FoxMediaTerm
{
    class VideoPlaybackTelemetry;

    struct VideoMediaInfo
    {
        int width = 0;
        int height = 0;
        std::optional<std::chrono::microseconds> duration;
        bool hasAudio = false;
    };

    class VideoDecoder
    {
      public:
        ~VideoDecoder();

        VideoDecoder(VideoDecoder&& other) noexcept;
        VideoDecoder& operator=(VideoDecoder&& other) noexcept;

        VideoDecoder(const VideoDecoder&) = delete;
        VideoDecoder& operator=(const VideoDecoder&) = delete;

        [[nodiscard]] static bool HasSupportedExtension(const std::filesystem::path& path);
        [[nodiscard]] static std::expected<VideoDecoder, std::string> Open(
            const std::filesystem::path& path,
            VideoPlaybackTelemetry* telemetry = nullptr
        );
        [[nodiscard]] static std::expected<VideoMediaInfo, std::string> Probe(const std::filesystem::path& path);
        [[nodiscard]] std::expected<std::optional<VideoFrame>, std::string> ReadFrame(
            int maximumWidth,
            int maximumHeight
        );

      private:
        struct Implementation;

        explicit VideoDecoder(std::unique_ptr<Implementation> implementation);

        std::unique_ptr<Implementation> m_Implementation;
    };
} // namespace FoxMediaTerm
