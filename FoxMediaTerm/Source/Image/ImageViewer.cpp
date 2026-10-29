#include "ImageViewer.h"

#include <algorithm>
#include <chrono>
#include <limits>
#include <string>
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
            ETerminalProtocol protocol,
            RenderableImage image,
            ImagePrepareMetrics* metrics,
            PngEncodeFunction pngEncoder
        );
        [[nodiscard]] std::string BuildPositionedPayload(
            ETerminalProtocol protocol,
            std::string encoded,
            int imageColumns,
            int imageRows,
            TerminalCellSize terminalSize,
            ImagePrepareMetrics* metrics
        );
    } // namespace

    std::expected<ImageView, std::string> ImageViewer::Prepare(
        RasterImage image,
        ETerminalProtocol protocol,
        TerminalCellSize terminalSize,
        TerminalCellPixelSize cellPixelSize,
        ImagePrepareMetrics* metrics,
        EKittyPreparationMode kittyMode,
        PngEncodeFunction pngEncoder
    )
    {
        const auto prepareStart = std::chrono::steady_clock::now();
        if (protocol == ETerminalProtocol::Auto)
        {
            return std::unexpected("The terminal image protocol was not resolved");
        }

        const bool usesPixelGraphics = UsesPixelGraphics(protocol);
        const int cellPixelWidth = std::max(cellPixelSize.width, 1);
        const int cellPixelHeight = std::max(cellPixelSize.height, 1);
        const int maximumWidth =
            usesPixelGraphics ? SafeMultiply(terminalSize.columns, cellPixelWidth) : terminalSize.columns;
        const int maximumHeight =
            usesPixelGraphics ? SafeMultiply(terminalSize.rows, cellPixelHeight) : SafeMultiply(terminalSize.rows, 2);

        const auto conversionStart = std::chrono::steady_clock::now();
        auto renderable = ImageResizer::ResizeToFit(std::move(image), maximumWidth, maximumHeight);
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

        const int imageColumns =
            usesPixelGraphics ? DivideRoundUp(renderable->width, cellPixelWidth) : renderable->width;
        const int imageRows = usesPixelGraphics ? DivideRoundUp(renderable->height, cellPixelHeight)
                                                : DivideRoundUp(renderable->height, 2);

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
            }

            std::string first = BuildPositionedPayload(
                protocol,
                std::move(encoded->first),
                imageColumns,
                imageRows,
                terminalSize,
                metrics
            );
            std::string second = BuildPositionedPayload(
                protocol,
                std::move(encoded->second),
                imageColumns,
                imageRows,
                terminalSize,
                nullptr
            );
            if (metrics != nullptr)
            {
                metrics->RecordStage(EImagePrepareStage::PrepareTotal, std::chrono::steady_clock::now() - prepareStart);
            }

            return ImageView {
                .payload = std::move(first),
                .alternatePayload = std::move(second),
                .kittyTransmitChunks = encoded->transmitChunks,
                .kittyDeleteSuffixBytes = encoded->deleteSuffixBytes,
                .usesAnsiUtf8 = false,
            };
        }

        const auto backendStart = std::chrono::steady_clock::now();
        auto encoded = Encode(protocol, std::move(*renderable), metrics, pngEncoder);
        const auto backendEnd = std::chrono::steady_clock::now();
        if (!encoded)
        {
            return std::unexpected(encoded.error());
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::BackendEncode, backendEnd - backendStart);
        }

        std::string payload =
            BuildPositionedPayload(protocol, std::move(*encoded), imageColumns, imageRows, terminalSize, metrics);
        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::PrepareTotal, std::chrono::steady_clock::now() - prepareStart);
        }

        return ImageView {
            .payload = std::move(payload),
            .usesAnsiUtf8 = !usesPixelGraphics,
        };
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
            const long long product = static_cast<long long>(left) * right;
            return static_cast<int>(std::min<long long>(product, std::numeric_limits<int>::max()));
        }

        [[nodiscard]] int DivideRoundUp(int value, int divisor)
        {
            return value / divisor + (value % divisor != 0 ? 1 : 0);
        }

        void AppendCursorPosition(std::string& payload, int row, int column)
        {
            payload += "\x1b[";
            payload += std::to_string(row + 1);
            payload.push_back(';');
            payload += std::to_string(column + 1);
            payload.push_back('H');
        }

        [[nodiscard]] std::expected<std::string, std::string> Encode(
            ETerminalProtocol protocol,
            RenderableImage image,
            ImagePrepareMetrics* metrics,
            PngEncodeFunction pngEncoder
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
                    return std::unexpected("The terminal image protocol was not resolved");
            }

            return std::unexpected("Unsupported terminal image protocol");
        }

        [[nodiscard]] std::string BuildPositionedPayload(
            ETerminalProtocol protocol,
            std::string encoded,
            int imageColumns,
            int imageRows,
            TerminalCellSize terminalSize,
            ImagePrepareMetrics* metrics
        )
        {
            const auto assemblyStart = std::chrono::steady_clock::now();
            const int imageColumn = std::max(0, (terminalSize.columns - imageColumns) / 2);
            const int imageRow = std::max(0, (terminalSize.rows - imageRows) / 2);
            std::string payload;
            std::size_t reallocations = 0;
            const auto appendTracked = [&payload, &reallocations](auto&& operation) {
                const std::size_t previousCapacity = payload.capacity();
                operation();
                reallocations += payload.capacity() != previousCapacity ? 1 : 0;
            };
            if (UsesPixelGraphics(protocol))
            {
                appendTracked([&payload, imageRow, imageColumn] {
                    AppendCursorPosition(payload, imageRow, imageColumn);
                });
                if (encoded.ends_with('\n'))
                {
                    encoded.pop_back();
                }
                appendTracked([&payload, &encoded] { payload += encoded; });
                if (metrics != nullptr)
                {
                    metrics->RecordStage(
                        EImagePrepareStage::FinalPayloadAssembly,
                        std::chrono::steady_clock::now() - assemblyStart
                    );
                    metrics->finalPayloadBytes = payload.size();
                    metrics->finalPayloadCapacity = payload.capacity();
                    metrics->finalPayloadReallocations = reallocations;
                }

                return payload;
            }

            std::size_t lineStart = 0;
            int rowOffset = 0;
            while (lineStart < encoded.size())
            {
                const std::size_t lineEnd = encoded.find('\n', lineStart);
                const std::size_t boundedEnd = lineEnd == std::string::npos ? encoded.size() : lineEnd;
                appendTracked([&payload, imageRow, rowOffset, imageColumn] {
                    AppendCursorPosition(payload, imageRow + rowOffset, imageColumn);
                });
                appendTracked([&payload, &encoded, lineStart, boundedEnd] {
                    payload.append(encoded, lineStart, boundedEnd - lineStart);
                });
                if (lineEnd == std::string::npos)
                {
                    break;
                }

                lineStart = lineEnd + 1;
                ++rowOffset;
            }

            if (metrics != nullptr)
            {
                metrics->RecordStage(
                    EImagePrepareStage::FinalPayloadAssembly,
                    std::chrono::steady_clock::now() - assemblyStart
                );
                metrics->finalPayloadBytes = payload.size();
                metrics->finalPayloadCapacity = payload.capacity();
                metrics->finalPayloadReallocations = reallocations;
            }

            return payload;
        }
    } // namespace
} // namespace FoxMediaTerm
