#pragma once

#include <chrono>
#include <optional>
#include <string>
#include <string_view>

namespace FoxMediaTerm
{
    struct TerminalTextSupport
    {
        bool interactive = false;
        bool unicode = false;
        bool trueColor = false;
        bool color256 = false;
    };

    class TerminalProbe
    {
      public:
        [[nodiscard]] static TerminalTextSupport DetectTextSupport();
        [[nodiscard]] static std::optional<std::string> Query(
            std::string_view request,
            std::chrono::milliseconds timeout
        );
    };
} // namespace FoxMediaTerm
