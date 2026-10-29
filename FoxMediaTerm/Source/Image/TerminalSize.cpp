#include "TerminalSize.h"

#ifdef _WIN32
#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <sys/ioctl.h>
#include <unistd.h>
#endif

namespace FoxMediaTerm
{
    std::optional<TerminalCellSize> TerminalSize::Detect()
    {
#ifdef _WIN32
        const HANDLE outputHandle = GetStdHandle(STD_OUTPUT_HANDLE);
        if (outputHandle == nullptr || outputHandle == INVALID_HANDLE_VALUE)
        {
            return std::nullopt;
        }

        CONSOLE_SCREEN_BUFFER_INFO information { };
        if (!GetConsoleScreenBufferInfo(outputHandle, &information))
        {
            return std::nullopt;
        }

        const int columns = information.srWindow.Right - information.srWindow.Left + 1;
        const int rows = information.srWindow.Bottom - information.srWindow.Top + 1;
#else
        winsize size { };
        if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) != 0)
        {
            return std::nullopt;
        }

        const int columns = size.ws_col;
        const int rows = size.ws_row;
#endif

        if (columns <= 0 || rows <= 0)
        {
            return std::nullopt;
        }

        return TerminalCellSize { columns, rows };
    }
} // namespace FoxMediaTerm
