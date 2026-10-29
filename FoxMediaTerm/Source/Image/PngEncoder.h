#pragma once

#include <cstdint>
#include <expected>
#include <string>
#include <vector>

#include "Image.h"

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    // Video may supply its own PNG encoder without changing still-image compression or terminal framing.
    using PngEncodeFunction = std::expected<std::vector<std::uint8_t>, std::string> (*)(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics
    );

    struct PngEncoderConfiguration
    {
        std::string implementation;
        int compressionLevel = 0;
        int forcedFilter = 0;
        std::string compressionStrategy;
        bool reusesContext = false;
        bool exposesFilterAndDeflateTimings = false;
        std::string cpuRequirement;
    };

    class PngEncoder
    {
      public:
        [[nodiscard]] static std::expected<std::vector<std::uint8_t>, std::string> Encode(
            const RenderableImage& image,
            ImagePrepareMetrics* metrics = nullptr
        );
        [[nodiscard]] static PngEncoderConfiguration Configuration();
    };
} // namespace FoxMediaTerm
