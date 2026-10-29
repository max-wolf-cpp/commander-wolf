#include "ImageRegionRenderer.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <limits>
#include <string_view>
#include <utility>

#include "AsciiGrayscaleRenderer.h"
#include "HalfBlock256Renderer.h"
#include "HalfBlockTrueColorRenderer.h"
#include "ITerm2Renderer.h"
#include "ImageResizer.h"
#include "KittyRenderer.h"
#include "SixelRenderer.h"

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] bool UsesPixelGraphics(ETerminalProtocol protocol);
        [[nodiscard]] int SafeMultiply(int left, int right);
        [[nodiscard]] int DivideRoundUp(int value, int divisor);
        void AppendCursorPosition(std::string& payload, int row, int column);
        [[nodiscard]] std::expected<std::string, std::string> Encode(
            RenderableImage image,
            ETerminalProtocol protocol,
            PngEncodeFunction pngEncoder,
            ImagePrepareMetrics* metrics
        );
        [[nodiscard]] std::string BuildPositionedPayload(
            ETerminalProtocol protocol,
            std::string encoded,
            ImageCellRegion region,
            int imageColumns,
            int imageRows,
            bool clearExistingKittyImage
        );
    } // namespace

    std::expected<ImageRegionView, std::string> ImageRegionRenderer::Prepare(
        RasterImage image,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region,
        PngEncodeFunction pngEncoder,
        ImagePrepareMetrics* metrics,
        EKittyPreparationMode kittyMode
    )
    {
        const auto prepareStart = std::chrono::steady_clock::now();
        if (region.columns <= 0 || region.rows <= 0)
        {
            return std::unexpected("The terminal image region must have a positive size");
        }

        const bool usesPixelGraphics = UsesPixelGraphics(protocol);
        const ImagePixelSize maximumSize = MaximumPixelSize(protocol, cellPixelSize, region);

        const auto conversionStart = std::chrono::steady_clock::now();
        auto renderable = ImageResizer::ResizeToFit(std::move(image), maximumSize.width, maximumSize.height);
        const auto conversionEnd = std::chrono::steady_clock::now();
        if (!renderable)
        {
            return std::unexpected(renderable.error());
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::InputConversion, conversionEnd - conversionStart);
            metrics->inputPixels =
                static_cast<std::uint64_t>(renderable->width) * static_cast<std::uint64_t>(renderable->height);
            metrics->inputBytes = renderable->pixels.size();
        }

        const int cellPixelWidth = std::max(1, cellPixelSize.width);
        const int cellPixelHeight = std::max(1, cellPixelSize.height);
        const int imageColumns =
            usesPixelGraphics ? DivideRoundUp(renderable->width, cellPixelWidth) : renderable->width;
        const int imageRows = usesPixelGraphics ? DivideRoundUp(renderable->height, cellPixelHeight)
                                                : DivideRoundUp(renderable->height, 2);
        const int pixelWidth = renderable->width;
        const int pixelHeight = renderable->height;

        if (protocol == ETerminalProtocol::Kitty && kittyMode == EKittyPreparationMode::PingPongVideo)
        {
            const auto backendStart = std::chrono::steady_clock::now();
            auto encoded = KittyRenderer::EncodePingPong(*renderable, metrics, pngEncoder);
            const auto backendEnd = std::chrono::steady_clock::now();
            if (!encoded)
            {
                return std::unexpected(encoded.error());
            }

            if (metrics != nullptr)
            {
                metrics->RecordStage(EImagePrepareStage::BackendEncode, backendEnd - backendStart);
                metrics->RecordStage(EImagePrepareStage::PrepareTotal, std::chrono::steady_clock::now() - prepareStart);
            }

            return ImageRegionView {
                .payload =
                    BuildPositionedPayload(protocol, std::move(encoded->first), region, imageColumns, imageRows, false),
                .alternatePayload = BuildPositionedPayload(
                    protocol,
                    std::move(encoded->second),
                    region,
                    imageColumns,
                    imageRows,
                    false
                ),
                .kittyTransmitChunks = encoded->transmitChunks,
                .kittyDeleteSuffixBytes = encoded->deleteSuffixBytes,
                .usesAnsiUtf8 = false,
                .pixelWidth = pixelWidth,
                .pixelHeight = pixelHeight,
            };
        }

        const auto backendStart = std::chrono::steady_clock::now();
        auto encoded = Encode(std::move(*renderable), protocol, pngEncoder, metrics);
        const auto backendEnd = std::chrono::steady_clock::now();
        if (!encoded)
        {
            return std::unexpected(encoded.error());
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::BackendEncode, backendEnd - backendStart);
            metrics->RecordStage(EImagePrepareStage::PrepareTotal, std::chrono::steady_clock::now() - prepareStart);
        }

        return ImageRegionView {
            .payload = BuildPositionedPayload(
                protocol,
                std::move(*encoded),
                region,
                imageColumns,
                imageRows,
                protocol == ETerminalProtocol::Kitty
            ),
            .usesAnsiUtf8 = !usesPixelGraphics,
            .pixelWidth = pixelWidth,
            .pixelHeight = pixelHeight,
        };
    }

    ImagePixelSize ImageRegionRenderer::MaximumPixelSize(
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    ) noexcept
    {
        const bool usesPixelGraphics = UsesPixelGraphics(protocol);
        const int cellPixelWidth = std::max(1, cellPixelSize.width);
        const int cellPixelHeight = std::max(1, cellPixelSize.height);

        return {
            usesPixelGraphics ? SafeMultiply(region.columns, cellPixelWidth) : region.columns,
            usesPixelGraphics ? SafeMultiply(region.rows, cellPixelHeight) : SafeMultiply(region.rows, 2),
        };
    }

    std::string ImageRegionRenderer::Cleanup(
        ETerminalProtocol protocol,
        ImageCellRegion region,
        std::array<std::uint8_t, 3> background
    )
    {
        if (region.columns <= 0 || region.rows <= 0)
        {
            return { };
        }

        std::string payload = "\x1b"
                              "7";
        if (protocol == ETerminalProtocol::Kitty)
        {
            payload += KittyRenderer::DeleteAll();
        }
        payload += "\x1b[48;2;" + std::to_string(background[0]) + ';' + std::to_string(background[1]) + ';'
                   + std::to_string(background[2]) + 'm';
        const std::string blankRow(static_cast<std::size_t>(region.columns), ' ');
        for (int rowOffset = 0; rowOffset < region.rows; ++rowOffset)
        {
            AppendCursorPosition(payload, region.row + rowOffset, region.column);
            payload += blankRow;
        }

        payload += "\x1b[0m\x1b"
                   "8";

        return payload;
    }

    namespace
    {
        [[nodiscard]] bool UsesPixelGraphics(ETerminalProtocol protocol)
        {
            return protocol == ETerminalProtocol::Kitty || protocol == ETerminalProtocol::ITerm2
                   || protocol == ETerminalProtocol::Sixel;
        }

        [[nodiscard]] int SafeMultiply(int left, int right)
        {
            const std::int64_t product = static_cast<std::int64_t>(left) * static_cast<std::int64_t>(right);
            return static_cast<int>(std::min<std::int64_t>(product, std::numeric_limits<int>::max()));
        }

        [[nodiscard]] int DivideRoundUp(int value, int divisor)
        {
            return (value + divisor - 1) / divisor;
        }

        void AppendCursorPosition(std::string& payload, int row, int column)
        {
            payload += "\x1b[" + std::to_string(row + 1) + ';' + std::to_string(column + 1) + 'H';
        }

        [[nodiscard]] std::expected<std::string, std::string> Encode(
            RenderableImage image,
            ETerminalProtocol protocol,
            PngEncodeFunction pngEncoder,
            ImagePrepareMetrics* metrics
        )
        {
            switch (protocol)
            {
                case ETerminalProtocol::ITerm2:
                    return ITerm2Renderer::Encode(image, { }, metrics, pngEncoder);
                case ETerminalProtocol::Sixel:
                    return SixelRenderer::Encode(std::move(image), metrics);
                case ETerminalProtocol::Kitty:
                    return KittyRenderer::Encode(image, metrics, pngEncoder);
                case ETerminalProtocol::HalfBlockTrueColor:
                    return HalfBlockTrueColorRenderer::Encode(image, metrics);
                case ETerminalProtocol::HalfBlock256:
                    return HalfBlock256Renderer::Encode(image, metrics);
                case ETerminalProtocol::AsciiGrayscale:
                    return AsciiGrayscaleRenderer::Encode(image, metrics);
                case ETerminalProtocol::Auto:
                    return std::unexpected("Automatic terminal protocol must be resolved before rendering a region");
            }

            return std::unexpected("Unsupported terminal image protocol");
        }

        [[nodiscard]] std::string BuildPositionedPayload(
            ETerminalProtocol protocol,
            std::string encoded,
            ImageCellRegion region,
            int imageColumns,
            int imageRows,
            bool clearExistingKittyImage
        )
        {
            const int imageColumn = region.column + std::max(0, (region.columns - imageColumns) / 2);
            const int imageRow = region.row + std::max(0, (region.rows - imageRows) / 2);
            std::string payload = "\x1b"
                                  "7";
            if (clearExistingKittyImage)
            {
                payload += KittyRenderer::DeleteAll();
            }

            if (UsesPixelGraphics(protocol))
            {
                AppendCursorPosition(payload, imageRow, imageColumn);
                if (encoded.ends_with('\n'))
                {
                    encoded.pop_back();
                }
                payload += encoded;
            }
            else
            {
                std::size_t lineStart = 0;
                int rowOffset = 0;
                while (lineStart < encoded.size())
                {
                    const std::size_t lineEnd = encoded.find('\n', lineStart);
                    const std::size_t boundedEnd = lineEnd == std::string::npos ? encoded.size() : lineEnd;
                    AppendCursorPosition(payload, imageRow + rowOffset, imageColumn);
                    payload.append(encoded, lineStart, boundedEnd - lineStart);
                    if (lineEnd == std::string::npos)
                    {
                        break;
                    }

                    lineStart = lineEnd + 1;
                    ++rowOffset;
                }
            }

            payload += "\x1b"
                       "8";

            return payload;
        }
    } // namespace
} // namespace FoxMediaTerm
