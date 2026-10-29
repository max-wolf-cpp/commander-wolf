#pragma once

#include <cstddef>
#include <cstdint>
#include <string_view>
#include <vector>

namespace FoxMediaTerm
{
    enum class EImagePixelFormat
    {
        Rgb24,
        Rgba32,
    };

    [[nodiscard]] constexpr std::size_t ChannelCount(EImagePixelFormat format)
    {
        return format == EImagePixelFormat::Rgba32 ? 4 : 3;
    }

    [[nodiscard]] constexpr bool HasAlpha(EImagePixelFormat format)
    {
        return format == EImagePixelFormat::Rgba32;
    }

    [[nodiscard]] constexpr std::string_view ImagePixelFormatName(EImagePixelFormat format)
    {
        return format == EImagePixelFormat::Rgba32 ? "rgba" : "rgb24";
    }

    struct RasterImage
    {
        int width = 0;
        int height = 0;
        std::vector<std::uint8_t> pixels;
        EImagePixelFormat pixelFormat = EImagePixelFormat::Rgba32;
    };

    using RgbaImage = RasterImage;

    struct RenderableImage
    {
        int width = 0;
        int height = 0;
        std::vector<std::uint8_t> pixels;
        EImagePixelFormat pixelFormat = EImagePixelFormat::Rgba32;
    };
} // namespace FoxMediaTerm
