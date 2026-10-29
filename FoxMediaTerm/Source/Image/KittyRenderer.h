#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <string>
#include <string_view>

#include "PngEncoder.h"

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    struct KittyPingPongPayloads
    {
        std::string first;
        std::string second;
        std::size_t transmitChunks = 0;
        std::size_t deleteSuffixBytes = 0;
    };

    class KittyRenderer
    {
      public:
        static constexpr std::uint32_t k_FirstImageId = 1'465'272'321;
        static constexpr std::uint32_t k_SecondImageId = 1'465'272'322;
        static constexpr std::uint32_t k_FirstPlacementId = 1'465'276'417;
        static constexpr std::uint32_t k_SecondPlacementId = 1'465'276'418;

        [[nodiscard]] static std::expected<std::string, std::string> Encode(
            const RenderableImage& image,
            ImagePrepareMetrics* metrics = nullptr,
            PngEncodeFunction pngEncoder = &PngEncoder::Encode
        );
        // Each video payload places its image before deleting the previously visible image.
        [[nodiscard]] static std::expected<KittyPingPongPayloads, std::string> EncodePingPong(
            const RenderableImage& image,
            ImagePrepareMetrics* metrics = nullptr,
            PngEncodeFunction pngEncoder = &PngEncoder::Encode
        );

        [[nodiscard]] static constexpr std::string_view DeleteAll()
        {
            return "\x1b_Ga=d,d=A\x1b\\";
        }
    };
} // namespace FoxMediaTerm
