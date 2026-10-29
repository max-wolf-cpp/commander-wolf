#pragma once

#include <expected>
#include <string>

#include "PngEncoder.h"

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    struct ITerm2RenderOptions
    {
        int displayWidthPixels = 0;
        int displayHeightPixels = 0;
        int displayColumns = 0;
        int displayRows = 0;
        bool preserveAspectRatio = true;
        bool preserveCursor = false;
    };

    class ITerm2Renderer
    {
      public:
        [[nodiscard]] static std::expected<std::string, std::string> Encode(
            const RenderableImage& image,
            ITerm2RenderOptions options = { },
            ImagePrepareMetrics* metrics = nullptr,
            PngEncodeFunction pngEncoder = &PngEncoder::Encode
        );
    };
} // namespace FoxMediaTerm
