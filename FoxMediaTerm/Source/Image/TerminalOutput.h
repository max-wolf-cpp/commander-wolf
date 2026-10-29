#pragma once

#include <expected>
#include <string>
#include <string_view>

namespace FoxMediaTerm
{
    class TerminalOutput
    {
      public:
        [[nodiscard]] static std::expected<void, std::string> WriteRaw(std::string_view text, bool flush = true);
        [[nodiscard]] static std::expected<void, std::string> WriteAnsiUtf8(
            std::string_view utf8Text,
            bool flush = true
        );
    };
} // namespace FoxMediaTerm
