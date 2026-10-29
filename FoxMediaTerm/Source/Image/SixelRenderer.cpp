#include "SixelRenderer.h"

#include <cstddef>
#include <limits>
#include <memory>
#include <string>
#include <string_view>

#include <sixel.h>

#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr int k_PaletteColors = 256;

        struct SixelOutput
        {
            std::string bytes;
            std::size_t reallocations = 0;
        };

        int AppendSixelOutput(char* data, int size, void* context)
        {
            auto& output = *static_cast<SixelOutput*>(context);
            const std::size_t previousCapacity = output.bytes.capacity();
            output.bytes.append(data, static_cast<std::size_t>(size));
            output.reallocations += output.bytes.capacity() != previousCapacity ? 1 : 0;
            return size;
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

        [[nodiscard]] std::string DescribeSixelError(std::string_view operation, SIXELSTATUS status)
        {
            const char* description = sixel_helper_format_error(status);
            return std::string(operation)
                   + " failed: " + (description == nullptr ? "unknown libsixel error" : description);
        }
    } // namespace

    std::expected<std::string, std::string> SixelRenderer::Encode(RenderableImage image, ImagePrepareMetrics* metrics)
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The renderable image has invalid dimensions or storage");
        }

        SixelOutput escapeStream;
        sixel_output_t* outputPointer = nullptr;
        SIXELSTATUS status = sixel_output_new(&outputPointer, AppendSixelOutput, &escapeStream, nullptr);
        if (SIXEL_FAILED(status))
        {
            return std::unexpected(DescribeSixelError("Creating SIXEL output", status));
        }

        std::unique_ptr<sixel_output_t, decltype(&sixel_output_unref)> output(outputPointer, sixel_output_unref);

        sixel_dither_t* ditherPointer = nullptr;
        status = sixel_dither_new(&ditherPointer, k_PaletteColors, nullptr);
        if (SIXEL_FAILED(status))
        {
            return std::unexpected(DescribeSixelError("Creating SIXEL dither", status));
        }

        std::unique_ptr<sixel_dither_t, decltype(&sixel_dither_unref)> dither(ditherPointer, sixel_dither_unref);

        const auto paletteStart = std::chrono::steady_clock::now();
        status = sixel_dither_initialize(
            dither.get(),
            image.pixels.data(),
            image.width,
            image.height,
            HasAlpha(image.pixelFormat) ? SIXEL_PIXELFORMAT_RGBA8888 : SIXEL_PIXELFORMAT_RGB888,
            LARGE_NORM,
            REP_AUTO,
            QUALITY_AUTO
        );
        if (SIXEL_FAILED(status))
        {
            return std::unexpected(DescribeSixelError("Initializing SIXEL palette", status));
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(
                EImagePrepareStage::SixelPaletteQuantization,
                std::chrono::steady_clock::now() - paletteStart
            );
        }

        const auto encodeStart = std::chrono::steady_clock::now();
        status = sixel_encode(image.pixels.data(), image.width, image.height, 0, dither.get(), output.get());
        if (SIXEL_FAILED(status))
        {
            return std::unexpected(DescribeSixelError("Encoding SIXEL image", status));
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::SixelDitherEncode, std::chrono::steady_clock::now() - encodeStart);
        }

        const std::size_t previousCapacity = escapeStream.bytes.capacity();
        escapeStream.bytes.push_back('\n');
        escapeStream.reallocations += escapeStream.bytes.capacity() != previousCapacity ? 1 : 0;
        if (metrics != nullptr)
        {
            metrics->RecordTemporaryBuffer(escapeStream.bytes.capacity(), escapeStream.reallocations);
        }

        return std::move(escapeStream.bytes);
    }

    SixelEncoderConfiguration SixelRenderer::Configuration()
    {
        return {
            .implementation = "libsixel " LIBSIXEL_VERSION,
            .paletteColors = k_PaletteColors,
            .largestDimensionAlgorithm = "LARGE_NORM",
            .representativeColorAlgorithm = "REP_AUTO",
            .qualityMode = "QUALITY_AUTO",
            .diffusionAlgorithm = "Floyd-Steinberg (libsixel context default)",
            .reusesContext = false,
            .exposesDitherAndPackingTimings = false,
        };
    }
} // namespace FoxMediaTerm
