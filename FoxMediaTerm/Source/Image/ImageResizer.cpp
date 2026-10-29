#include "ImageResizer.h"

#include <cstddef>
#include <cstdint>
#include <limits>
#include <utility>

#define STB_IMAGE_RESIZE_IMPLEMENTATION
#include <stb_image_resize2.h>

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] bool HasValidStorage(const RasterImage& image)
        {
            if (image.width <= 0 || image.height <= 0)
            {
                return false;
            }

            const std::size_t width = static_cast<std::size_t>(image.width);
            const std::size_t height = static_cast<std::size_t>(image.height);
            const std::size_t channels = ChannelCount(image.pixelFormat);
            return width <= std::numeric_limits<std::size_t>::max() / height / channels
                   && image.pixels.size() == width * height * channels;
        }
    } // namespace

    std::expected<RenderableImage, std::string> ImageResizer::Prepare(RasterImage image)
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The raster image has invalid dimensions or storage");
        }

        return RenderableImage {
            image.width,
            image.height,
            std::move(image.pixels),
            image.pixelFormat,
        };
    }

    std::expected<RenderableImage, std::string> ImageResizer::Resize(const RasterImage& image, int width, int height)
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The raster image has invalid dimensions or storage");
        }

        if (width <= 0 || height <= 0)
        {
            return std::unexpected("The render size must be positive");
        }

        const std::size_t destinationWidth = static_cast<std::size_t>(width);
        const std::size_t destinationHeight = static_cast<std::size_t>(height);
        const std::size_t channels = ChannelCount(image.pixelFormat);
        if (destinationWidth > std::numeric_limits<std::size_t>::max() / destinationHeight / channels)
        {
            return std::unexpected("The render size is too large");
        }

        RenderableImage resized {
            width,
            height,
            std::vector<std::uint8_t>(destinationWidth * destinationHeight * channels),
            image.pixelFormat,
        };

        const unsigned char* result = stbir_resize_uint8_linear(
            image.pixels.data(),
            image.width,
            image.height,
            0,
            resized.pixels.data(),
            width,
            height,
            0,
            HasAlpha(image.pixelFormat) ? STBIR_RGBA : STBIR_RGB
        );
        if (result == nullptr)
        {
            return std::unexpected("stb_image_resize2 failed to resize the image");
        }

        return resized;
    }

    std::expected<RenderableImage, std::string> ImageResizer::ResizeToFit(
        RasterImage image,
        int maximumWidth,
        int maximumHeight
    )
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The raster image has invalid dimensions or storage");
        }

        if (maximumWidth <= 0 || maximumHeight <= 0)
        {
            return std::unexpected("The maximum render size must be positive");
        }

        if (image.width <= maximumWidth && image.height <= maximumHeight)
        {
            return Prepare(std::move(image));
        }

        const std::uint64_t sourceWidth = static_cast<std::uint64_t>(image.width);
        const std::uint64_t sourceHeight = static_cast<std::uint64_t>(image.height);
        const std::uint64_t widthLimit = static_cast<std::uint64_t>(maximumWidth);
        const std::uint64_t heightLimit = static_cast<std::uint64_t>(maximumHeight);

        int targetWidth = maximumWidth;
        int targetHeight = maximumHeight;
        if (widthLimit * sourceHeight <= heightLimit * sourceWidth)
        {
            targetHeight = static_cast<int>((sourceHeight * widthLimit) / sourceWidth);
            targetHeight = targetHeight > 0 ? targetHeight : 1;
        }
        else
        {
            targetWidth = static_cast<int>((sourceWidth * heightLimit) / sourceHeight);
            targetWidth = targetWidth > 0 ? targetWidth : 1;
        }

        return Resize(image, targetWidth, targetHeight);
    }
} // namespace FoxMediaTerm
