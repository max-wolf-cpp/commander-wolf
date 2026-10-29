#pragma once

#include <cstdint>
#include <expected>
#include <string>
#include <vector>

#include "Image.h"
#include "PngEncoder.h"

namespace FoxMediaTerm
{
    class VideoPngEncoder
    {
      public:
        [[nodiscard]] static std::expected<std::vector<std::uint8_t>, std::string> Encode(
            const RenderableImage& image,
            ImagePrepareMetrics* metrics = nullptr
        );
        [[nodiscard]] static PngEncoderConfiguration Configuration();
    };
} // namespace FoxMediaTerm
