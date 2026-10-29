#include "TerminalGeometry.h"

#include <array>
#include <charconv>
#include <chrono>
#include <cstddef>
#include <string_view>

#include "TerminalProbe.h"

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
    namespace
    {
        constexpr std::chrono::milliseconds k_QueryTimeout { 250 };
        constexpr std::string_view k_GeometryQuery = "\x1b[16t\x1b[14t";

        struct PixelReport
        {
            int kind = 0;
            int height = 0;
            int width = 0;
        };

        [[nodiscard]] std::optional<PixelReport> ParseReportAt(std::string_view response, std::size_t offset)
        {
            if (offset + 2 >= response.size() || response[offset] != '\x1b' || response[offset + 1] != '[')
            {
                return std::nullopt;
            }

            std::array<int, 3> values { };
            std::size_t valueIndex = 0;
            std::size_t cursor = offset + 2;
            while (valueIndex < values.size())
            {
                const char* begin = response.data() + cursor;
                const char* end = response.data() + response.size();
                const auto [next, conversionError] = std::from_chars(begin, end, values[valueIndex]);
                if (conversionError != std::errc { })
                {
                    return std::nullopt;
                }

                cursor = static_cast<std::size_t>(next - response.data());
                ++valueIndex;
                if (valueIndex < values.size())
                {
                    if (cursor >= response.size() || response[cursor] != ';')
                    {
                        return std::nullopt;
                    }

                    ++cursor;
                }
            }

            if (cursor >= response.size() || response[cursor] != 't')
            {
                return std::nullopt;
            }

            return PixelReport { values[0], values[1], values[2] };
        }

        [[nodiscard]] std::optional<TerminalCellPixelSize> DetectPlatformGeometry(TerminalCellSize terminalSize)
        {
#ifdef _WIN32
            const HANDLE outputHandle = GetStdHandle(STD_OUTPUT_HANDLE);
            if (outputHandle == nullptr || outputHandle == INVALID_HANDLE_VALUE)
            {
                return std::nullopt;
            }

            CONSOLE_FONT_INFOEX fontInformation { };
            fontInformation.cbSize = sizeof(fontInformation);
            if (!GetCurrentConsoleFontEx(outputHandle, FALSE, &fontInformation) || fontInformation.dwFontSize.X < 4
                || fontInformation.dwFontSize.Y < 8)
            {
                return std::nullopt;
            }

            return TerminalCellPixelSize { fontInformation.dwFontSize.X, fontInformation.dwFontSize.Y, true };
#else
            winsize size { };
            if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) != 0 || size.ws_xpixel == 0 || size.ws_ypixel == 0
                || terminalSize.columns <= 0 || terminalSize.rows <= 0)
            {
                return std::nullopt;
            }

            const int cellWidth = size.ws_xpixel / terminalSize.columns;
            const int cellHeight = size.ws_ypixel / terminalSize.rows;
            if (cellWidth <= 0 || cellHeight <= 0)
            {
                return std::nullopt;
            }

            return TerminalCellPixelSize { cellWidth, cellHeight, true };
#endif
        }
    } // namespace

    TerminalCellPixelSize TerminalGeometry::Detect(TerminalCellSize terminalSize)
    {
        if (const auto response = TerminalProbe::Query(k_GeometryQuery, k_QueryTimeout))
        {
            if (const auto geometry = ParseResponse(*response, terminalSize))
            {
                return *geometry;
            }
        }

        if (const auto geometry = DetectPlatformGeometry(terminalSize))
        {
            return *geometry;
        }

        return { };
    }

    std::optional<TerminalCellPixelSize> TerminalGeometry::ParseResponse(
        std::string_view response,
        TerminalCellSize terminalSize
    )
    {
        std::optional<PixelReport> windowReport;
        for (std::size_t offset = 0; offset < response.size(); ++offset)
        {
            const auto report = ParseReportAt(response, offset);
            if (!report || report->width <= 0 || report->height <= 0)
            {
                continue;
            }

            if (report->kind == 6)
            {
                return TerminalCellPixelSize { report->width, report->height, true };
            }

            if (report->kind == 4)
            {
                windowReport = report;
            }
        }

        if (!windowReport || terminalSize.columns <= 0 || terminalSize.rows <= 0)
        {
            return std::nullopt;
        }

        const int cellWidth = windowReport->width / terminalSize.columns;
        const int cellHeight = windowReport->height / terminalSize.rows;
        if (cellWidth <= 0 || cellHeight <= 0)
        {
            return std::nullopt;
        }

        return TerminalCellPixelSize { cellWidth, cellHeight, true };
    }
} // namespace FoxMediaTerm
