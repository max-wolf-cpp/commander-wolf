#pragma once

#include <optional>

namespace FoxMediaTerm
{
    struct TerminalCellSize
    {
        int columns = 0;
        int rows = 0;
    };

    class TerminalSize
    {
      public:
        [[nodiscard]] static std::optional<TerminalCellSize> Detect();
    };
} // namespace FoxMediaTerm
