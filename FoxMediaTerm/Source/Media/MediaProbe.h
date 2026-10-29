#pragma once

#include <chrono>
#include <expected>
#include <filesystem>
#include <optional>
#include <string>

namespace FoxMediaTerm
{
    enum class EMediaStreamKind
    {
        Video,
        Audio,
    };

    struct ProbedMediaInfo
    {
        EMediaStreamKind kind = EMediaStreamKind::Video;
        int width = 0;
        int height = 0;
        std::optional<std::chrono::microseconds> duration;
        bool hasAudio = false;
    };

    class MediaProbe
    {
      public:
        [[nodiscard]] static bool HasSupportedExtension(const std::filesystem::path& path);
        [[nodiscard]] static std::expected<ProbedMediaInfo, std::string> Probe(const std::filesystem::path& path);
    };
} // namespace FoxMediaTerm
