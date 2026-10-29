#pragma once

#include <expected>
#include <string>

#include "Image.h"

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    class AsciiGrayscaleRenderer
    {
      public:
        [[nodiscard]] static std::expected<std::string, std::string> Encode(
            const RenderableImage& image,
            ImagePrepareMetrics* metrics = nullptr
        );
    };
} // namespace FoxMediaTerm
