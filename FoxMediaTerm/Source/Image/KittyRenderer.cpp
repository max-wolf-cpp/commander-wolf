#include "KittyRenderer.h"

#include <algorithm>
#include <cstddef>
#include <string>
#include <string_view>

#include "Base64Encoder.h"
#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::size_t k_MaxChunkSize = 4096;
        constexpr std::string_view k_CommandStart = "\x1b_G";
        constexpr std::string_view k_CommandEnd = "\x1b\\";

        [[nodiscard]] std::string BuildTransmission(
            std::string_view payload,
            std::uint32_t imageId,
            std::size_t& reallocations,
            std::size_t& transmitChunks
        )
        {
            std::string escapeStream;
            std::size_t previousCapacity = escapeStream.capacity();
            escapeStream.reserve(payload.size() + ((payload.size() / k_MaxChunkSize) + 1) * 48);
            reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;

            std::size_t offset = 0;
            bool firstChunk = true;
            while (offset < payload.size())
            {
                previousCapacity = escapeStream.capacity();
                const std::size_t chunkSize = std::min(k_MaxChunkSize, payload.size() - offset);
                const bool hasMoreChunks = offset + chunkSize < payload.size();

                escapeStream += k_CommandStart;
                if (firstChunk)
                {
                    escapeStream += "a=t,f=100,q=2,i=";
                    escapeStream += std::to_string(imageId);
                    escapeStream.push_back(',');
                    firstChunk = false;
                }
                escapeStream += hasMoreChunks ? "m=1;" : "m=0;";
                escapeStream.append(payload, offset, chunkSize);
                escapeStream += k_CommandEnd;
                reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;
                offset += chunkSize;
                ++transmitChunks;
            }

            return escapeStream;
        }

        void AppendPlace(std::string& payload, std::uint32_t imageId, std::uint32_t placementId)
        {
            payload += "\x1b_Ga=p,i=";
            payload += std::to_string(imageId);
            payload += ",p=";
            payload += std::to_string(placementId);
            payload += ",q=2\x1b\\";
        }

        [[nodiscard]] std::size_t AppendDelete(std::string& payload, std::uint32_t imageId)
        {
            const std::size_t start = payload.size();
            payload += "\x1b_Ga=d,d=I,i=";
            payload += std::to_string(imageId);
            payload += ",q=2\x1b\\";

            return payload.size() - start;
        }
    } // namespace

    std::expected<std::string, std::string> KittyRenderer::Encode(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics,
        PngEncodeFunction pngEncoder
    )
    {
        auto png = pngEncoder(image, metrics);
        if (!png)
        {
            return std::unexpected(png.error());
        }

        const std::string payload = Base64Encoder::Encode(*png, metrics);
        std::string escapeStream;
        std::size_t reallocations = 0;
        std::size_t previousCapacity = escapeStream.capacity();
        escapeStream.reserve(payload.size() + ((payload.size() / k_MaxChunkSize) + 1) * 24);
        reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;

        const auto framingStart = std::chrono::steady_clock::now();
        std::size_t offset = 0;
        bool firstChunk = true;
        while (offset < payload.size())
        {
            previousCapacity = escapeStream.capacity();
            const std::size_t chunkSize = std::min(k_MaxChunkSize, payload.size() - offset);
            const bool hasMoreChunks = offset + chunkSize < payload.size();

            escapeStream += k_CommandStart;
            if (firstChunk)
            {
                escapeStream += "a=T,f=100,";
                firstChunk = false;
            }
            escapeStream += hasMoreChunks ? "m=1;" : "m=0;";
            escapeStream.append(payload, offset, chunkSize);
            escapeStream += k_CommandEnd;
            reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;

            offset += chunkSize;
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(
                EImagePrepareStage::KittyFramingAssembly,
                std::chrono::steady_clock::now() - framingStart
            );
            metrics->RecordTemporaryBuffer(escapeStream.capacity(), reallocations);
        }

        return escapeStream;
    }

    std::expected<KittyPingPongPayloads, std::string> KittyRenderer::EncodePingPong(
        const RenderableImage& image,
        ImagePrepareMetrics* metrics,
        PngEncodeFunction pngEncoder
    )
    {
        auto png = pngEncoder(image, metrics);
        if (!png)
        {
            return std::unexpected(png.error());
        }

        const std::string encoded = Base64Encoder::Encode(*png, metrics);
        const auto framingStart = std::chrono::steady_clock::now();
        std::size_t reallocations = 0;
        std::size_t firstChunks = 0;
        std::string first = BuildTransmission(encoded, k_FirstImageId, reallocations, firstChunks);
        AppendPlace(first, k_FirstImageId, k_FirstPlacementId);
        const std::size_t firstDeleteBytes = AppendDelete(first, k_SecondImageId);

        std::size_t secondChunks = 0;
        std::string second = BuildTransmission(encoded, k_SecondImageId, reallocations, secondChunks);
        AppendPlace(second, k_SecondImageId, k_SecondPlacementId);
        const std::size_t secondDeleteBytes = AppendDelete(second, k_FirstImageId);

        if (metrics != nullptr)
        {
            metrics->RecordStage(
                EImagePrepareStage::KittyFramingAssembly,
                std::chrono::steady_clock::now() - framingStart
            );
            metrics->RecordTemporaryBuffer(first.capacity() + second.capacity(), reallocations);
        }

        return KittyPingPongPayloads {
            .first = std::move(first),
            .second = std::move(second),
            .transmitChunks = std::max(firstChunks, secondChunks),
            .deleteSuffixBytes = std::max(firstDeleteBytes, secondDeleteBytes),
        };
    }
} // namespace FoxMediaTerm
