#include "HalfBlock256Renderer.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <string>
#include <string_view>

#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::string_view k_UpperHalfBlock = "\xE2\x96\x80";
        constexpr std::array<int, 6> k_ColorCubeLevels { 0, 95, 135, 175, 215, 255 };

        struct RgbColor
        {
            std::uint8_t red = 0;
            std::uint8_t green = 0;
            std::uint8_t blue = 0;
        };

        [[nodiscard]] bool HasValidStorage(const RenderableImage& image)
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

        [[nodiscard]] RgbColor ReadColor(const RenderableImage& image, int x, int y)
        {
            const std::size_t pixelOffset =
                (static_cast<std::size_t>(y) * static_cast<std::size_t>(image.width) + static_cast<std::size_t>(x))
                * ChannelCount(image.pixelFormat);
            const std::uint16_t alpha = HasAlpha(image.pixelFormat) ? image.pixels[pixelOffset + 3] : 255;
            const auto compositeChannel = [alpha](std::uint8_t channel) {
                return static_cast<std::uint8_t>((static_cast<std::uint16_t>(channel) * alpha + 127) / 255);
            };

            return RgbColor {
                compositeChannel(image.pixels[pixelOffset]),
                compositeChannel(image.pixels[pixelOffset + 1]),
                compositeChannel(image.pixels[pixelOffset + 2]),
            };
        }

        [[nodiscard]] int ColorDistance(const RgbColor& color, int red, int green, int blue)
        {
            const int redDifference = static_cast<int>(color.red) - red;
            const int greenDifference = static_cast<int>(color.green) - green;
            const int blueDifference = static_cast<int>(color.blue) - blue;

            return redDifference * redDifference + greenDifference * greenDifference + blueDifference * blueDifference;
        }

        [[nodiscard]] int FindNearestPaletteIndex(const RgbColor& color)
        {
            int nearestIndex = 16;
            int nearestDistance = std::numeric_limits<int>::max();

            for (int redIndex = 0; redIndex < static_cast<int>(k_ColorCubeLevels.size()); ++redIndex)
            {
                for (int greenIndex = 0; greenIndex < static_cast<int>(k_ColorCubeLevels.size()); ++greenIndex)
                {
                    for (int blueIndex = 0; blueIndex < static_cast<int>(k_ColorCubeLevels.size()); ++blueIndex)
                    {
                        const int distance = ColorDistance(
                            color,
                            k_ColorCubeLevels[redIndex],
                            k_ColorCubeLevels[greenIndex],
                            k_ColorCubeLevels[blueIndex]
                        );
                        if (distance < nearestDistance)
                        {
                            nearestDistance = distance;
                            nearestIndex = 16 + 36 * redIndex + 6 * greenIndex + blueIndex;
                        }
                    }
                }
            }

            for (int grayscaleIndex = 0; grayscaleIndex < 24; ++grayscaleIndex)
            {
                const int level = 8 + 10 * grayscaleIndex;
                const int distance = ColorDistance(color, level, level, level);
                if (distance < nearestDistance)
                {
                    nearestDistance = distance;
                    nearestIndex = 232 + grayscaleIndex;
                }
            }

            return nearestIndex;
        }

        void AppendColor(std::string& output, std::string_view ansiPrefix, const RgbColor& color)
        {
            output += ansiPrefix;
            output += std::to_string(FindNearestPaletteIndex(color));
            output.push_back('m');
        }
    } // namespace

    std::expected<std::string, std::string> HalfBlock256Renderer::Encode(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics
    )
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The renderable image has invalid dimensions or storage");
        }

        const std::size_t terminalRows = (static_cast<std::size_t>(image.height) + 1) / 2;
        std::string output;
        std::size_t reallocations = 0;
        std::size_t previousCapacity = output.capacity();
        const std::size_t terminalCells = static_cast<std::size_t>(image.width) * terminalRows;
        if (terminalCells <= std::numeric_limits<std::size_t>::max() / 32)
        {
            output.reserve(terminalCells * 32);
            reallocations += output.capacity() != previousCapacity ? 1 : 0;
            previousCapacity = output.capacity();
        }

        const auto cellsStart = std::chrono::steady_clock::now();
        for (int topY = 0; topY < image.height; topY += 2)
        {
            const bool hasBottomRow = topY + 1 < image.height;
            for (int x = 0; x < image.width; ++x)
            {
                AppendColor(output, "\x1b[38;5;", ReadColor(image, x, topY));
                if (hasBottomRow)
                {
                    AppendColor(output, "\x1b[48;5;", ReadColor(image, x, topY + 1));
                }
                else
                {
                    output += "\x1b[49m";
                }
                output += k_UpperHalfBlock;
            }
            output += "\x1b[0m\n";
            const std::size_t currentCapacity = output.capacity();
            reallocations += currentCapacity != previousCapacity ? 1 : 0;
            previousCapacity = currentCapacity;
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(
                EImagePrepareStage::HalfBlock256CellsAnsiAssembly,
                std::chrono::steady_clock::now() - cellsStart
            );
            metrics->RecordTemporaryBuffer(output.capacity(), reallocations);
        }

        return output;
    }
} // namespace FoxMediaTerm
