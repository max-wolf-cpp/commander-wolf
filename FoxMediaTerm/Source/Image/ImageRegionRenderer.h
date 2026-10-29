#pragma once

#include <array>
#include <cstdint>
#include <expected>
#include <string>

#include "Image.h"
#include "ImagePrepareMetrics.h"
#include "ImageViewer.h"
#include "PngEncoder.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"

namespace FoxMediaTerm
{
    struct ImageCellRegion
    {
        int column = 0;
        int row = 0;
        int columns = 0;
        int rows = 0;

        bool operator==(const ImageCellRegion&) const = default;
    };

    struct ImageRegionView
    {
        std::string payload;
        std::string alternatePayload;
        std::size_t kittyTransmitChunks = 0;
        std::size_t kittyDeleteSuffixBytes = 0;
        bool usesAnsiUtf8 = false;
        int pixelWidth = 0;
        int pixelHeight = 0;
    };

    struct ImagePixelSize
    {
        int width = 0;
        int height = 0;
    };

    class ImageRegionRenderer
    {
      public:
        [[nodiscard]] static std::expected<ImageRegionView, std::string> Prepare(
            RasterImage image,
            ETerminalProtocol protocol,
            TerminalCellPixelSize cellPixelSize,
            ImageCellRegion region,
            PngEncodeFunction pngEncoder = &PngEncoder::Encode,
            ImagePrepareMetrics* metrics = nullptr,
            EKittyPreparationMode kittyMode = EKittyPreparationMode::SingleImage
        );
        [[nodiscard]] static ImagePixelSize MaximumPixelSize(
            ETerminalProtocol protocol,
            TerminalCellPixelSize cellPixelSize,
            ImageCellRegion region
        ) noexcept;
        [[nodiscard]] static std::string Cleanup(
            ETerminalProtocol protocol,
            ImageCellRegion region,
            std::array<std::uint8_t, 3> background
        );
    };
} // namespace FoxMediaTerm
