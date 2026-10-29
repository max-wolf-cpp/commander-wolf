#pragma once

#include <expected>
#include <string>

#include "Image.h"

namespace FoxMediaTerm
{
    class ImageResizer
    {
      public:
        [[nodiscard]] static std::expected<RenderableImage, std::string> Prepare(RasterImage image);
        [[nodiscard]] static std::expected<RenderableImage, std::string> Resize(
            const RasterImage& image,
            int width,
            int height
        );
        [[nodiscard]] static std::expected<RenderableImage, std::string> ResizeToFit(
            RasterImage image,
            int maximumWidth,
            int maximumHeight
        );
    };
} // namespace FoxMediaTerm
