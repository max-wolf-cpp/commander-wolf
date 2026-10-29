#pragma once

#include <expected>
#include <optional>
#include <string>

#include "Image.h"

namespace FoxMediaTerm
{
    class VideoDecoder;

    class VideoThumbnail
    {
      public:
        [[nodiscard]] static bool IsMeaningful(const RasterImage& image) noexcept;
        [[nodiscard]] static std::expected<std::optional<RasterImage>, std::string> Read(
            VideoDecoder& decoder,
            int maximumWidth,
            int maximumHeight
        );
    };
} // namespace FoxMediaTerm
