#pragma once

#include <optional>
#include <string_view>

#include "TerminalSize.h"

namespace FoxMediaTerm
{
    struct TerminalCellPixelSize
    {
        int width = 8;
        int height = 16;
        bool exact = false;
    };

    class TerminalGeometry
    {
      public:
        [[nodiscard]] static TerminalCellPixelSize Detect(TerminalCellSize terminalSize);
        [[nodiscard]] static std::optional<TerminalCellPixelSize> ParseResponse(
            std::string_view response,
            TerminalCellSize terminalSize
        );
    };
} // namespace FoxMediaTerm
