#include "VideoPngEncoder.h"

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <mutex>
#include <vector>

#include <fpng.h>
#include <fpnge.h>

#include "ImagePrepareMetrics.h"

#if defined(_MSC_VER)
#include <intrin.h>
#endif

namespace FoxMediaTerm
{
    namespace
    {
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

        [[nodiscard]] bool CpuSupportsAvx2()
        {
#if FOX_MEDIA_TERM_HAS_FPNGE && defined(_MSC_VER) && (defined(_M_X64) || defined(_M_IX86))
            std::array<int, 4> registers { };
            __cpuid(registers.data(), 1);
            constexpr int k_OsXsave = 1 << 27;
            constexpr int k_Avx = 1 << 28;
            if ((registers[2] & (k_OsXsave | k_Avx)) != (k_OsXsave | k_Avx) || (_xgetbv(0) & 0x6) != 0x6)
            {
                return false;
            }

            __cpuidex(registers.data(), 7, 0);

            return (registers[1] & (1 << 5)) != 0;
#elif FOX_MEDIA_TERM_HAS_FPNGE && (defined(__x86_64__) || defined(__i386__))                                           \
    && (defined(__GNUC__) || defined(__clang__))
            return __builtin_cpu_supports("avx2");
#else
            return false;
#endif
        }

        [[nodiscard]] bool UseFpnge()
        {
            static const bool k_UseFpnge = CpuSupportsAvx2();
            return k_UseFpnge;
        }

        [[nodiscard]] std::expected<std::vector<std::uint8_t>, std::string> EncodeFpng(const RenderableImage& image)
        {
            static std::once_flag initialization;
            std::call_once(initialization, fpng::fpng_init);

            std::vector<std::uint8_t> output;
            if (!fpng::fpng_encode_image_to_memory(
                    image.pixels.data(),
                    static_cast<std::uint32_t>(image.width),
                    static_cast<std::uint32_t>(image.height),
                    static_cast<std::uint32_t>(ChannelCount(image.pixelFormat)),
                    output,
                    0
                ))
            {
                return std::unexpected("fpng 1-pass failed to encode the video frame");
            }

            return output;
        }

        [[nodiscard]] std::expected<std::vector<std::uint8_t>, std::string> EncodeFpnge(const RenderableImage& image)
        {
#if FOX_MEDIA_TERM_HAS_FPNGE
            const std::size_t channels = ChannelCount(image.pixelFormat);
            FPNGEOptions options { };
            FPNGEFillOptions(&options, FPNGE_COMPRESS_LEVEL_DEFAULT, FPNGE_CICP_NONE);
            std::vector<std::uint8_t> output(FPNGEOutputAllocSize(1, channels, image.width, image.height));
            const std::size_t outputSize = FPNGEEncode(
                1,
                channels,
                image.pixels.data(),
                image.width,
                static_cast<std::size_t>(image.width) * channels,
                image.height,
                output.data(),
                &options
            );
            if (outputSize == 0 || outputSize > output.size())
            {
                return std::unexpected("fpnge default failed to encode the video frame");
            }

            output.resize(outputSize);

            return output;
#else
            static_cast<void>(image);
            return std::unexpected("fpnge is unavailable on this architecture");
#endif
        }
    } // namespace

    std::expected<std::vector<std::uint8_t>, std::string> VideoPngEncoder::Encode(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics
    )
    {
        if (!HasValidStorage(image))
        {
            return std::unexpected("The renderable video frame has invalid dimensions or storage");
        }

        const auto encodeStart = std::chrono::steady_clock::now();
        std::expected<std::vector<std::uint8_t>, std::string> output =
            UseFpnge() ? EncodeFpnge(image) : EncodeFpng(image);
        if (!output && UseFpnge())
        {
            output = EncodeFpng(image);
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::PngEncode, std::chrono::steady_clock::now() - encodeStart);
            if (output)
            {
                metrics->RecordTemporaryBuffer(output->capacity(), 0);
            }
        }

        return output;
    }

    PngEncoderConfiguration VideoPngEncoder::Configuration()
    {
        if (UseFpnge())
        {
            return {
                .implementation = "fpnge default",
                .compressionLevel = FPNGE_COMPRESS_LEVEL_DEFAULT,
                .forcedFilter = -1,
                .compressionStrategy = "fpnge default realtime preset",
                .reusesContext = false,
                .exposesFilterAndDeflateTimings = false,
                .cpuRequirement = "AVX2 fast path; fpng 1-pass on encoding failure",
            };
        }

        return {
            .implementation = "fpng 1-pass",
            .compressionLevel = 0,
            .forcedFilter = 2,
            .compressionStrategy = "fpng single-pass portable fallback",
            .reusesContext = true,
            .exposesFilterAndDeflateTimings = false,
            .cpuRequirement = "portable scalar fallback",
        };
    }
} // namespace FoxMediaTerm
