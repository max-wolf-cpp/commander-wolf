#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <utility>
#include <vector>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "AbiBoundary.h"
#include "Image.h"
#include "ImageRegionRenderer.h"
#include "MediaPreview.h"
#include "PngEncoder.h"
#include "VideoPngEncoder.h"

namespace
{
    using FoxMediaTerm::ImageRegionRenderer;
    using FoxMediaTerm::RasterImage;

    [[nodiscard]] RasterImage CopyRaster(
        const std::uint8_t* pixels,
        std::size_t pixelsSize,
        int width,
        int height,
        std::size_t stride,
        FoxMediaTermPixelFormat pixelFormat
    );
} // namespace

extern "C"
{
    FoxMediaTermStatus FoxMediaTermPreparePreview(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        FoxMediaTermKind* kind,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    )
    {
        Reset(buffer);

        return Guard(error, [=] {
            if (kind == nullptr || buffer == nullptr)
            {
                throw std::invalid_argument("The media preview output is null");
            }

            const ETerminalProtocol resolvedProtocol = ProtocolFromAbi(protocol);
            const TerminalCellPixelSize cellSize = CellSize(cellPixelWidth, cellPixelHeight);
            const ImageCellRegion imageRegion = RegionFromAbi(region);
            auto preview = FoxMediaTerm::Bridge::LoadPreview(
                PathFromUtf8(pathUtf8, pathSize),
                resolvedProtocol,
                cellSize,
                imageRegion
            );
            if (!preview)
            {
                Store(error, preview.error());
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            auto prepared = ImageRegionRenderer::Prepare(
                std::move(preview->first),
                resolvedProtocol,
                cellSize,
                imageRegion,
                preview->second == FOX_MEDIA_TERM_KIND_VIDEO ? &FoxMediaTerm::VideoPngEncoder::Encode
                                                             : &FoxMediaTerm::PngEncoder::Encode
            );
            if (!prepared)
            {
                Store(error, prepared.error());
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            *kind = preview->second;
            Store(*buffer, std::move(prepared->payload), prepared->usesAnsiUtf8);

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }

    FoxMediaTermStatus FoxMediaTermPrepareRaster(
        const uint8_t* pixels,
        size_t pixelsSize,
        int32_t width,
        int32_t height,
        size_t stride,
        FoxMediaTermPixelFormat pixelFormat,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    )
    {
        Reset(buffer);

        return Guard(error, [=] {
            if (buffer == nullptr)
            {
                throw std::invalid_argument("The raster output is null");
            }

            auto prepared = ImageRegionRenderer::Prepare(
                CopyRaster(pixels, pixelsSize, width, height, stride, pixelFormat),
                ProtocolFromAbi(protocol),
                CellSize(cellPixelWidth, cellPixelHeight),
                RegionFromAbi(region),
                &FoxMediaTerm::PngEncoder::Encode
            );
            if (!prepared)
            {
                Store(error, prepared.error());
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            Store(*buffer, std::move(prepared->payload), prepared->usesAnsiUtf8);

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }

    FoxMediaTermStatus FoxMediaTermPrepareCleanup(
        FoxMediaTermProtocol protocol,
        FoxMediaTermRegion region,
        uint8_t backgroundRed,
        uint8_t backgroundGreen,
        uint8_t backgroundBlue,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    )
    {
        Reset(buffer);

        return Guard(error, [=] {
            if (buffer == nullptr)
            {
                throw std::invalid_argument("The media cleanup output is null");
            }

            Store(
                *buffer,
                ImageRegionRenderer::Cleanup(
                    ProtocolFromAbi(protocol),
                    RegionFromAbi(region),
                    std::array<std::uint8_t, 3> { backgroundRed, backgroundGreen, backgroundBlue }
                ),
                false
            );

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }
}

namespace
{
    [[nodiscard]] RasterImage CopyRaster(
        const std::uint8_t* pixels,
        std::size_t pixelsSize,
        int width,
        int height,
        std::size_t stride,
        FoxMediaTermPixelFormat pixelFormat
    )
    {
        if (pixels == nullptr)
        {
            throw std::invalid_argument("The raster pixel buffer is null");
        }

        if (width <= 0 || height <= 0)
        {
            throw std::invalid_argument("Raster dimensions must be positive");
        }

        if (pixelFormat != FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8)
        {
            throw std::invalid_argument("The raster pixel format is unsupported");
        }

        constexpr std::size_t k_ChannelCount = 4;
        const std::size_t unsignedWidth = static_cast<std::size_t>(width);
        const std::size_t unsignedHeight = static_cast<std::size_t>(height);
        if (unsignedWidth > std::numeric_limits<std::size_t>::max() / k_ChannelCount)
        {
            throw std::invalid_argument("The raster row size overflows");
        }

        const std::size_t rowSize = unsignedWidth * k_ChannelCount;
        if (stride < rowSize)
        {
            throw std::invalid_argument("The raster stride is shorter than one RGBA row");
        }

        if (unsignedHeight > std::numeric_limits<std::size_t>::max() / stride || pixelsSize < stride * unsignedHeight)
        {
            throw std::invalid_argument("The raster pixel buffer is too short");
        }

        if (unsignedHeight > std::numeric_limits<std::size_t>::max() / rowSize)
        {
            throw std::invalid_argument("The normalized raster size overflows");
        }

        RasterImage image {
            .width = width,
            .height = height,
            .pixels = std::vector<std::uint8_t>(rowSize * unsignedHeight),
            .pixelFormat = FoxMediaTerm::EImagePixelFormat::Rgba32,
        };
        for (std::size_t row = 0; row < unsignedHeight; ++row)
        {
            std::copy_n(pixels + row * stride, rowSize, image.pixels.data() + row * rowSize);
        }

        return image;
    }
} // namespace
