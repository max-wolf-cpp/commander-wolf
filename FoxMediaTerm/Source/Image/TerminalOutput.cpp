#include "TerminalOutput.h"

#include <algorithm>
#include <iostream>
#include <limits>
#include <string>

#ifdef _WIN32
#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#endif

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] std::expected<void, std::string> WriteBytes(std::string_view text, bool flush)
        {
            if (text.size() > static_cast<std::size_t>(std::numeric_limits<std::streamsize>::max()))
            {
                return std::unexpected("The terminal output stream is too large");
            }

            std::cout.write(text.data(), static_cast<std::streamsize>(text.size()));
            if (flush)
            {
                std::cout.flush();
            }

            if (!std::cout)
            {
                return std::unexpected("Unable to write the terminal output stream");
            }

            return { };
        }

#ifdef _WIN32
        class ConsoleModeRestorer
        {
          public:
            ConsoleModeRestorer(HANDLE outputHandle, DWORD originalMode)
                : m_OutputHandle(outputHandle)
                , m_OriginalMode(originalMode)
            {
            }

            ~ConsoleModeRestorer()
            {
                SetConsoleMode(m_OutputHandle, m_OriginalMode);
            }

            ConsoleModeRestorer(const ConsoleModeRestorer&) = delete;
            ConsoleModeRestorer& operator=(const ConsoleModeRestorer&) = delete;

          private:
            HANDLE m_OutputHandle;
            DWORD m_OriginalMode;
        };

        [[nodiscard]] std::string WindowsError(std::string_view operation)
        {
            return std::string(operation) + " failed with Windows error " + std::to_string(GetLastError());
        }

        [[nodiscard]] std::expected<void, std::string> WriteWindowsBytes(
            HANDLE outputHandle,
            DWORD originalMode,
            std::string_view text
        )
        {
            const DWORD outputMode = originalMode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if (!SetConsoleMode(outputHandle, outputMode))
            {
                return std::unexpected(WindowsError("Enabling virtual terminal output"));
            }

            const ConsoleModeRestorer modeRestorer(outputHandle, originalMode);

            constexpr DWORD k_MaxWriteBytes = 32768;
            std::size_t offset = 0;
            while (offset < text.size())
            {
                const DWORD bytesToWrite =
                    static_cast<DWORD>(std::min<std::size_t>(k_MaxWriteBytes, text.size() - offset));
                DWORD bytesWritten = 0;
                if (!WriteFile(outputHandle, text.data() + offset, bytesToWrite, &bytesWritten, nullptr))
                {
                    return std::unexpected(WindowsError("Writing terminal protocol bytes"));
                }

                if (bytesWritten == 0)
                {
                    return std::unexpected("Writing terminal protocol bytes made no progress");
                }

                offset += bytesWritten;
            }

            return { };
        }

        [[nodiscard]] std::expected<std::wstring, std::string> Utf8ToWide(std::string_view text)
        {
            if (text.empty())
            {
                return std::wstring { };
            }

            if (text.size() > static_cast<std::size_t>(std::numeric_limits<int>::max()))
            {
                return std::unexpected("The terminal output is too large for Windows UTF-8 conversion");
            }

            const int inputSize = static_cast<int>(text.size());
            const int wideSize = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), inputSize, nullptr, 0);
            if (wideSize == 0)
            {
                return std::unexpected(WindowsError("Converting terminal output from UTF-8"));
            }

            std::wstring wideText(static_cast<std::size_t>(wideSize), L'\0');
            if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(), inputSize, wideText.data(), wideSize)
                != wideSize)
            {
                return std::unexpected(WindowsError("Converting terminal output from UTF-8"));
            }

            return wideText;
        }

        [[nodiscard]] std::expected<void, std::string> WriteWindowsConsole(
            HANDLE outputHandle,
            DWORD originalMode,
            std::string_view text
        )
        {
            auto wideText = Utf8ToWide(text);
            if (!wideText)
            {
                return std::unexpected(wideText.error());
            }

            const DWORD outputMode = originalMode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if (!SetConsoleMode(outputHandle, outputMode))
            {
                return std::unexpected(WindowsError("Enabling virtual terminal output"));
            }

            const ConsoleModeRestorer modeRestorer(outputHandle, originalMode);

            constexpr DWORD k_MaxWriteCharacters = 32768;
            std::size_t offset = 0;
            while (offset < wideText->size())
            {
                const DWORD charactersToWrite =
                    static_cast<DWORD>(std::min<std::size_t>(k_MaxWriteCharacters, wideText->size() - offset));
                DWORD charactersWritten = 0;
                if (!WriteConsoleW(
                        outputHandle,
                        wideText->data() + offset,
                        charactersToWrite,
                        &charactersWritten,
                        nullptr
                    ))
                {
                    return std::unexpected(WindowsError("Writing Unicode terminal output"));
                }

                if (charactersWritten == 0)
                {
                    return std::unexpected("Writing Unicode terminal output made no progress");
                }

                offset += charactersWritten;
            }

            return { };
        }
#endif
    } // namespace

    std::expected<void, std::string> TerminalOutput::WriteRaw(std::string_view text, bool flush)
    {
#ifdef _WIN32
        const HANDLE outputHandle = GetStdHandle(STD_OUTPUT_HANDLE);
        DWORD originalMode = 0;
        if (outputHandle != nullptr && outputHandle != INVALID_HANDLE_VALUE
            && GetConsoleMode(outputHandle, &originalMode))
        {
            static_cast<void>(flush);
            return WriteWindowsBytes(outputHandle, originalMode, text);
        }
#endif
        return WriteBytes(text, flush);
    }

    std::expected<void, std::string> TerminalOutput::WriteAnsiUtf8(std::string_view utf8Text, bool flush)
    {
#ifdef _WIN32
        HANDLE outputHandle = GetStdHandle(STD_OUTPUT_HANDLE);
        if (outputHandle == nullptr || outputHandle == INVALID_HANDLE_VALUE)
        {
            return std::unexpected(WindowsError("Getting the standard output handle"));
        }

        DWORD originalMode = 0;
        if (GetConsoleMode(outputHandle, &originalMode))
        {
            static_cast<void>(flush);
            return WriteWindowsConsole(outputHandle, originalMode, utf8Text);
        }
#endif

        return WriteBytes(utf8Text, flush);
    }
} // namespace FoxMediaTerm
