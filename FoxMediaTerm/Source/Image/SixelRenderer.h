#pragma once

#include <expected>
#include <string>

#include "Image.h"

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    struct SixelEncoderConfiguration
    {
        std::string implementation;
        int paletteColors = 0;
        std::string largestDimensionAlgorithm;
        std::string representativeColorAlgorithm;
        std::string qualityMode;
        std::string diffusionAlgorithm;
        bool reusesContext = false;
        bool exposesDitherAndPackingTimings = false;
    };

    class SixelRenderer
    {
      public:
        [[nodiscard]] static std::expected<std::string, std::string> Encode(
            RenderableImage image,
            ImagePrepareMetrics* metrics = nullptr
        );
        [[nodiscard]] static SixelEncoderConfiguration Configuration();
    };
} // namespace FoxMediaTerm
