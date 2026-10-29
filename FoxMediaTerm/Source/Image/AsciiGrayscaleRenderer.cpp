#include "AsciiGrayscaleRenderer.h"

#include <cstddef>
#include <limits>
#include <string>
#include <string_view>

#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::string_view k_GrayscaleRamp = " .:-=+*#%@";

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

        [[nodiscard]] int ReadLuminance(const RenderableImage& image, int x, int y)
        {
            const std::size_t pixelOffset =
                (static_cast<std::size_t>(y) * static_cast<std::size_t>(image.width) + static_cast<std::size_t>(x))
                * ChannelCount(image.pixelFormat);
            const int alpha = HasAlpha(image.pixelFormat) ? image.pixels[pixelOffset + 3] : 255;
            const int red = (image.pixels[pixelOffset] * alpha + 127) / 255;
            const int green = (image.pixels[pixelOffset + 1] * alpha + 127) / 255;
            const int blue = (image.pixels[pixelOffset + 2] * alpha + 127) / 255;

            return (54 * red + 183 * green + 19 * blue + 128) / 256;
        }

        [[nodiscard]] char LuminanceCharacter(int luminance)
        {
            const int maximumRampIndex = static_cast<int>(k_GrayscaleRamp.size()) - 1;
            const int rampIndex = (luminance * maximumRampIndex + 127) / 255;

            return k_GrayscaleRamp[static_cast<std::size_t>(rampIndex)];
        }
    } // namespace

    std::expected<std::string, std::string> AsciiGrayscaleRenderer::Encode(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics
    )
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The renderable image has invalid dimensions or storage");
        }

        const std::size_t terminalRows = (static_cast<std::size_t>(image.height) + 1) / 2;
        const std::size_t rowSize = static_cast<std::size_t>(image.width) + 1;
        std::string output;
        std::size_t reallocations = 0;
        std::size_t previousCapacity = output.capacity();
        if (terminalRows <= std::numeric_limits<std::size_t>::max() / rowSize)
        {
            output.reserve(terminalRows * rowSize);
            reallocations += output.capacity() != previousCapacity ? 1 : 0;
            previousCapacity = output.capacity();
        }

        const auto cellsStart = std::chrono::steady_clock::now();
        for (int topY = 0; topY < image.height; topY += 2)
        {
            const bool hasBottomRow = topY + 1 < image.height;
            for (int x = 0; x < image.width; ++x)
            {
                int luminance = ReadLuminance(image, x, topY);
                if (hasBottomRow)
                {
                    luminance = (luminance + ReadLuminance(image, x, topY + 1) + 1) / 2;
                }
                output.push_back(LuminanceCharacter(luminance));
            }
            output.push_back('\n');
            const std::size_t currentCapacity = output.capacity();
            reallocations += currentCapacity != previousCapacity ? 1 : 0;
            previousCapacity = currentCapacity;
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::AsciiCellsAssembly, std::chrono::steady_clock::now() - cellsStart);
            metrics->RecordTemporaryBuffer(output.capacity(), reallocations);
        }

        return output;
    }
} // namespace FoxMediaTerm
