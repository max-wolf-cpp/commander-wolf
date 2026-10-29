#pragma once

#include <cstdint>
#include <span>
#include <string>

namespace FoxMediaTerm
{
    struct ImagePrepareMetrics;

    class Base64Encoder
    {
      public:
        [[nodiscard]] static std::string Encode(
            std::span<const std::uint8_t> input,
            ImagePrepareMetrics* metrics = nullptr
        );
    };
} // namespace FoxMediaTerm
