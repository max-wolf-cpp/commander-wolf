#include "Base64Encoder.h"

#include <cstddef>
#include <cstdint>

#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr char k_Alphabet[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    }

    std::string Base64Encoder::Encode(std::span<const std::uint8_t> input, ImagePrepareMetrics* metrics)
    {
        std::string output;
        const auto encodeStart = std::chrono::steady_clock::now();
        const std::size_t previousCapacity = output.capacity();
        output.reserve(((input.size() + 2) / 3) * 4);
        std::size_t reallocations = output.capacity() != previousCapacity ? 1 : 0;

        std::size_t index = 0;
        while (index + 3 <= input.size())
        {
            const std::uint32_t value = (static_cast<std::uint32_t>(input[index]) << 16)
                                        | (static_cast<std::uint32_t>(input[index + 1]) << 8)
                                        | static_cast<std::uint32_t>(input[index + 2]);
            output.push_back(k_Alphabet[(value >> 18) & 0x3F]);
            output.push_back(k_Alphabet[(value >> 12) & 0x3F]);
            output.push_back(k_Alphabet[(value >> 6) & 0x3F]);
            output.push_back(k_Alphabet[value & 0x3F]);
            index += 3;
        }

        const std::size_t remaining = input.size() - index;
        if (remaining == 1)
        {
            const std::uint32_t value = static_cast<std::uint32_t>(input[index]) << 16;
            output.push_back(k_Alphabet[(value >> 18) & 0x3F]);
            output.push_back(k_Alphabet[(value >> 12) & 0x3F]);
            output.append("==");
        }
        else if (remaining == 2)
        {
            const std::uint32_t value =
                (static_cast<std::uint32_t>(input[index]) << 16) | (static_cast<std::uint32_t>(input[index + 1]) << 8);
            output.push_back(k_Alphabet[(value >> 18) & 0x3F]);
            output.push_back(k_Alphabet[(value >> 12) & 0x3F]);
            output.push_back(k_Alphabet[(value >> 6) & 0x3F]);
            output.push_back('=');
        }

        if (metrics != nullptr)
        {
            metrics->RecordStage(EImagePrepareStage::Base64Encode, std::chrono::steady_clock::now() - encodeStart);
            metrics->RecordTemporaryBuffer(output.capacity(), reallocations);
        }

        return output;
    }
} // namespace FoxMediaTerm
