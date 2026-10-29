#include "PngEncoder.h"

#include <cstddef>
#include <cstdint>
#include <limits>
#include <vector>

#include "ImagePrepareMetrics.h"

#define STB_IMAGE_WRITE_IMPLEMENTATION
#include <stb_image_write.h>

namespace FoxMediaTerm
{
    namespace
    {
        struct PngOutput
        {
            std::vector<std::uint8_t> bytes;
            std::size_t reallocations = 0;
            std::chrono::steady_clock::duration copyDuration { };
        };

        void AppendPngBytes(void* context, void* data, int size)
        {
            auto& output = *static_cast<PngOutput*>(context);
            const auto copyStart = std::chrono::steady_clock::now();
            const auto* first = static_cast<const std::uint8_t*>(data);
            const std::size_t previousCapacity = output.bytes.capacity();
            output.bytes.insert(output.bytes.end(), first, first + size);
            output.reallocations += output.bytes.capacity() != previousCapacity ? 1 : 0;
            output.copyDuration += std::chrono::steady_clock::now() - copyStart;
        }

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
    } // namespace

    std::expected<std::vector<std::uint8_t>, std::string> PngEncoder::Encode(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics
    )
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The renderable image has invalid dimensions or storage");
        }

        PngOutput png;
        const auto encodeStart = std::chrono::steady_clock::now();
        const int channels = static_cast<int>(ChannelCount(image.pixelFormat));
        const int stride = image.width * channels;
        if (stbi_write_png_to_func(
                AppendPngBytes,
                &png,
                image.width,
                image.height,
                channels,
                image.pixels.data(),
                stride
            )
            == 0)
        {
            return std::unexpected("stb_image_write failed to encode the image as PNG");
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::PngEncode, std::chrono::steady_clock::now() - encodeStart);
            metrics->RecordStage(EImagePrepareStage::PngOutputCopy, png.copyDuration);
            metrics->RecordTemporaryBuffer(png.bytes.capacity(), png.reallocations);
        }

        return std::move(png.bytes);
    }

    PngEncoderConfiguration PngEncoder::Configuration()
    {
        return {
            .implementation = "stb_image_write",
            .compressionLevel = stbi_write_png_compression_level,
            .forcedFilter = stbi_write_force_png_filter,
            .compressionStrategy = "stb built-in DEFLATE (no configurable strategy)",
            .reusesContext = false,
            .exposesFilterAndDeflateTimings = false,
        };
    }
} // namespace FoxMediaTerm
