#pragma once

#include <expected>
#include <filesystem>
#include <string>
#include <utility>

#include "Image.h"

namespace FoxMediaTerm
{
    class ImageLoader
    {
      public:
        [[nodiscard]] static bool HasSupportedExtension(const std::filesystem::path& path);
        [[nodiscard]] static bool IsSupported(const std::filesystem::path& path);
        [[nodiscard]] static std::expected<std::pair<int, int>, std::string> ProbeDimensions(
            const std::filesystem::path& path
        );
        [[nodiscard]] static std::expected<RgbaImage, std::string> Load(const std::filesystem::path& path);
    };
} // namespace FoxMediaTerm
