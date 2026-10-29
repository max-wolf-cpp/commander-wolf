#include "VideoThumbnail.h"

#include <algorithm>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <utility>

#include "VideoDecoder.h"
#include "VideoFrame.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::size_t k_MaximumScannedFrames = 90;
        constexpr std::chrono::microseconds k_MaximumScanDuration { 3'000'000 };
        constexpr std::uint8_t k_VisibleAlphaThreshold = 16;
        constexpr std::uint8_t k_VisibleLuminanceThreshold = 24;
        constexpr std::size_t k_MeaningfulPixelDivisor = 50;

        [[nodiscard]] std::uint8_t Luminance(const std::uint8_t* pixel) noexcept
        {
            return static_cast<std::uint8_t>((54U * pixel[0] + 183U * pixel[1] + 19U * pixel[2]) / 256U);
        }

        [[nodiscard]] std::uint64_t FrameScore(const RasterImage& image) noexcept
        {
            const std::size_t channels = ChannelCount(image.pixelFormat);
            if (image.width <= 0 || image.height <= 0 || channels < 3)
            {
                return 0;
            }

            std::uint64_t score = 0;
            for (std::size_t offset = 0; offset + channels <= image.pixels.size(); offset += channels)
            {
                const std::uint8_t alpha = HasAlpha(image.pixelFormat) ? image.pixels[offset + 3] : 255;
                if (alpha < k_VisibleAlphaThreshold)
                {
                    continue;
                }

                score += static_cast<std::uint64_t>(Luminance(image.pixels.data() + offset)) * alpha;
            }

            return score;
        }
    } // namespace

    bool VideoThumbnail::IsMeaningful(const RasterImage& image) noexcept
    {
        const std::size_t channels = ChannelCount(image.pixelFormat);
        if (image.width <= 0 || image.height <= 0 || channels < 3)
        {
            return false;
        }

        const std::size_t pixelCount = image.pixels.size() / channels;
        const std::size_t requiredVisiblePixels =
            std::max<std::size_t>(1, (pixelCount + k_MeaningfulPixelDivisor - 1) / k_MeaningfulPixelDivisor);
        std::size_t visiblePixels = 0;
        for (std::size_t offset = 0; offset + channels <= image.pixels.size(); offset += channels)
        {
            const std::uint8_t alpha = HasAlpha(image.pixelFormat) ? image.pixels[offset + 3] : 255;
            if (alpha >= k_VisibleAlphaThreshold
                && Luminance(image.pixels.data() + offset) >= k_VisibleLuminanceThreshold)
            {
                ++visiblePixels;
                if (visiblePixels >= requiredVisiblePixels)
                {
                    return true;
                }
            }
        }

        return false;
    }

    std::expected<std::optional<RasterImage>, std::string> VideoThumbnail::Read(
        VideoDecoder& decoder,
        int maximumWidth,
        int maximumHeight
    )
    {
        std::optional<RasterImage> bestFrame;
        std::uint64_t bestScore = 0;
        std::optional<std::chrono::microseconds> firstTimestamp;
        for (std::size_t frameIndex = 0; frameIndex < k_MaximumScannedFrames; ++frameIndex)
        {
            auto decoded = decoder.ReadFrame(maximumWidth, maximumHeight);
            if (!decoded)
            {
                return std::unexpected(decoded.error());
            }

            if (!*decoded)
            {
                break;
            }

            VideoFrame frame = std::move(**decoded);
            if (!firstTimestamp)
            {
                firstTimestamp = frame.timestamp;
            }

            if (IsMeaningful(frame.image))
            {
                return std::optional<RasterImage>(std::move(frame.image));
            }

            const std::uint64_t score = FrameScore(frame.image);
            if (!bestFrame || score > bestScore)
            {
                bestScore = score;
                bestFrame = std::move(frame.image);
            }

            if (frame.timestamp - *firstTimestamp >= k_MaximumScanDuration)
            {
                break;
            }
        }

        return bestFrame;
    }
} // namespace FoxMediaTerm
