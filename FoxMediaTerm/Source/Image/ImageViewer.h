#pragma once

#include <expected>
#include <string>

#include "Image.h"
#include "ImagePrepareMetrics.h"
#include "PngEncoder.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"
#include "TerminalSize.h"

namespace FoxMediaTerm
{
    enum class EKittyPreparationMode
    {
        SingleImage,
        PingPongVideo,
    };

    struct ImageView
    {
        std::string payload;
        std::string alternatePayload;
        std::size_t kittyTransmitChunks = 0;
        std::size_t kittyDeleteSuffixBytes = 0;
        bool usesAnsiUtf8 = false;
    };

    class ImageViewer
    {
      public:
        [[nodiscard]] static std::expected<ImageView, std::string> Prepare(
            RasterImage image,
            ETerminalProtocol protocol,
            TerminalCellSize terminalSize,
            TerminalCellPixelSize cellPixelSize,
            ImagePrepareMetrics* metrics = nullptr,
            EKittyPreparationMode kittyMode = EKittyPreparationMode::SingleImage,
            PngEncodeFunction pngEncoder = &PngEncoder::Encode
        );
    };
} // namespace FoxMediaTerm
