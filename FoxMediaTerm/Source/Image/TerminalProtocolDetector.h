#pragma once

#include <string_view>

#include "TerminalProtocol.h"

namespace FoxMediaTerm
{
    struct TerminalCapabilities
    {
        bool interactive = false;
        bool kittyGraphics = false;
        bool iTerm2Graphics = false;
        bool sixelGraphics = false;
        bool unicode = false;
        bool trueColor = false;
        bool color256 = false;
    };

    class TerminalProtocolDetector
    {
      public:
        [[nodiscard]] static ETerminalProtocol Detect();
        [[nodiscard]] static TerminalCapabilities Probe();
        [[nodiscard]] static TerminalCapabilities ParseProbeResponse(std::string_view response);
        [[nodiscard]] static ETerminalProtocol Select(const TerminalCapabilities& capabilities);
    };
} // namespace FoxMediaTerm
