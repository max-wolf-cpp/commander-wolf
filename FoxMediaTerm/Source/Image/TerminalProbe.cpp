#include "TerminalProbe.h"

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <limits>
#include <string>
#include <string_view>
#include <vector>

#ifdef _WIN32
#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <cerrno>

#include <fcntl.h>
#include <poll.h>
#include <termios.h>
#include <unistd.h>
#endif

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::size_t k_MaximumResponseSize = 64 * 1024;
        constexpr std::chrono::milliseconds k_ResponseIdleTimeout { 50 };

        [[nodiscard]] std::string EnvironmentValue(const char* name);
        [[nodiscard]] std::string Lowercase(std::string value);
        [[nodiscard]] bool ContainsUtf8(std::string_view value);
        [[nodiscard]] std::optional<std::string> QueryNativeTerminal(
            std::string_view request,
            std::chrono::milliseconds timeout
        );
    } // namespace

    TerminalTextSupport TerminalProbe::DetectTextSupport()
    {
        TerminalTextSupport support;

#ifdef _WIN32
        const HANDLE outputHandle = GetStdHandle(STD_OUTPUT_HANDLE);
        DWORD outputMode = 0;
        if (outputHandle == nullptr || outputHandle == INVALID_HANDLE_VALUE
            || !GetConsoleMode(outputHandle, &outputMode))
        {
            return support;
        }

        const DWORD probeMode = outputMode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        if (!SetConsoleMode(outputHandle, probeMode))
        {
            return support;
        }

        SetConsoleMode(outputHandle, outputMode);

        support.interactive = true;
        support.unicode = true;
        support.trueColor = true;
        support.color256 = true;
#else
        if (!isatty(STDOUT_FILENO))
        {
            return support;
        }

        const std::string term = Lowercase(EnvironmentValue("TERM"));
        if (term == "dumb")
        {
            return support;
        }

        support.interactive = true;

        std::string locale = EnvironmentValue("LC_ALL");
        if (locale.empty())
        {
            locale = EnvironmentValue("LC_CTYPE");
        }
        if (locale.empty())
        {
            locale = EnvironmentValue("LANG");
        }
        support.unicode = ContainsUtf8(Lowercase(std::move(locale)));

        const std::string colorTerm = Lowercase(EnvironmentValue("COLORTERM"));
        support.trueColor = colorTerm == "truecolor" || colorTerm == "24bit" || term.contains("direct")
                            || term.contains("truecolor") || term.contains("24bit");
        support.color256 = support.trueColor || term.contains("256color");
#endif

        return support;
    }

    std::optional<std::string> TerminalProbe::Query(std::string_view request, std::chrono::milliseconds timeout)
    {
        if (timeout <= std::chrono::milliseconds::zero() || !DetectTextSupport().interactive)
        {
            return std::nullopt;
        }

        return QueryNativeTerminal(request, timeout);
    }

    namespace
    {
        [[nodiscard]] std::string EnvironmentValue(const char* name)
        {
#ifdef _WIN32
            char* value = nullptr;
            std::size_t valueSize = 0;
            if (_dupenv_s(&value, &valueSize, name) != 0 || value == nullptr)
            {
                return { };
            }

            std::string result(value);
            std::free(value);
            return result;
#else
            const char* value = std::getenv(name);
            return value == nullptr ? std::string { } : std::string(value);
#endif
        }

        [[nodiscard]] std::string Lowercase(std::string value)
        {
            std::ranges::transform(value, value.begin(), [](unsigned char character) {
                return static_cast<char>(std::tolower(character));
            });
            return value;
        }

        [[nodiscard]] bool ContainsUtf8(std::string_view value)
        {
            return value.find("utf-8") != std::string_view::npos || value.find("utf8") != std::string_view::npos;
        }

        [[nodiscard]] int WaitDurationMilliseconds(
            std::chrono::steady_clock::time_point deadline,
            bool receivedResponse
        )
        {
            const auto now = std::chrono::steady_clock::now();
            if (now >= deadline)
            {
                return 0;
            }

            auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - now);
            if (remaining.count() == 0)
            {
                remaining = std::chrono::milliseconds(1);
            }
            if (receivedResponse)
            {
                remaining = std::min(remaining, k_ResponseIdleTimeout);
            }

            return static_cast<int>(std::min<std::int64_t>(remaining.count(), std::numeric_limits<int>::max()));
        }

#ifdef _WIN32
        class WindowsHandle
        {
          public:
            explicit WindowsHandle(HANDLE handle)
                : m_Handle(handle)
            {
            }

            ~WindowsHandle()
            {
                if (IsValid())
                {
                    CloseHandle(m_Handle);
                }
            }

            WindowsHandle(const WindowsHandle&) = delete;
            WindowsHandle& operator=(const WindowsHandle&) = delete;

            [[nodiscard]] bool IsValid() const
            {
                return m_Handle != nullptr && m_Handle != INVALID_HANDLE_VALUE;
            }

            [[nodiscard]] HANDLE Get() const
            {
                return m_Handle;
            }

          private:
            HANDLE m_Handle = INVALID_HANDLE_VALUE;
        };

        class WindowsConsoleModeRestorer
        {
          public:
            WindowsConsoleModeRestorer(HANDLE inputHandle, DWORD inputMode, HANDLE outputHandle, DWORD outputMode)
                : m_InputHandle(inputHandle)
                , m_InputMode(inputMode)
                , m_OutputHandle(outputHandle)
                , m_OutputMode(outputMode)
            {
            }

            ~WindowsConsoleModeRestorer()
            {
                SetConsoleMode(m_InputHandle, m_InputMode);
                SetConsoleMode(m_OutputHandle, m_OutputMode);
            }

            WindowsConsoleModeRestorer(const WindowsConsoleModeRestorer&) = delete;
            WindowsConsoleModeRestorer& operator=(const WindowsConsoleModeRestorer&) = delete;

          private:
            HANDLE m_InputHandle;
            DWORD m_InputMode;
            HANDLE m_OutputHandle;
            DWORD m_OutputMode;
        };

        [[nodiscard]] bool WriteAll(HANDLE outputHandle, std::string_view request)
        {
            std::size_t offset = 0;
            while (offset < request.size())
            {
                const DWORD writeSize = static_cast<DWORD>(
                    std::min<std::size_t>(request.size() - offset, std::numeric_limits<DWORD>::max())
                );
                DWORD written = 0;
                if (!WriteFile(outputHandle, request.data() + offset, writeSize, &written, nullptr) || written == 0)
                {
                    return false;
                }

                offset += written;
            }

            return true;
        }

        [[nodiscard]] bool HasReadableCharacter(HANDLE inputHandle)
        {
            DWORD eventCount = 0;
            if (!GetNumberOfConsoleInputEvents(inputHandle, &eventCount) || eventCount == 0)
            {
                return false;
            }

            constexpr DWORD k_MaximumPeekRecords = 4096;
            const DWORD recordsToPeek = std::min(eventCount, k_MaximumPeekRecords);
            std::vector<INPUT_RECORD> records(recordsToPeek);
            DWORD recordsRead = 0;
            if (!PeekConsoleInputW(inputHandle, records.data(), recordsToPeek, &recordsRead))
            {
                return false;
            }

            for (DWORD index = 0; index < recordsRead; ++index)
            {
                const INPUT_RECORD& record = records[index];
                if (record.EventType == KEY_EVENT && record.Event.KeyEvent.bKeyDown
                    && record.Event.KeyEvent.uChar.UnicodeChar != L'\0')
                {
                    return true;
                }
            }

            return false;
        }

        [[nodiscard]] std::optional<std::string> QueryNativeTerminal(
            std::string_view request,
            std::chrono::milliseconds timeout
        )
        {
            WindowsHandle inputHandle(CreateFileW(
                L"CONIN$",
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                nullptr,
                OPEN_EXISTING,
                0,
                nullptr
            ));
            WindowsHandle outputHandle(CreateFileW(
                L"CONOUT$",
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                nullptr,
                OPEN_EXISTING,
                0,
                nullptr
            ));
            if (!inputHandle.IsValid() || !outputHandle.IsValid())
            {
                return std::nullopt;
            }

            DWORD originalInputMode = 0;
            DWORD originalOutputMode = 0;
            if (!GetConsoleMode(inputHandle.Get(), &originalInputMode)
                || !GetConsoleMode(outputHandle.Get(), &originalOutputMode))
            {
                return std::nullopt;
            }

            if (HasReadableCharacter(inputHandle.Get()))
            {
                return std::nullopt;
            }

            const DWORD inputMode =
                (originalInputMode & ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT)) | ENABLE_VIRTUAL_TERMINAL_INPUT;
            const DWORD outputMode = originalOutputMode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if (!SetConsoleMode(inputHandle.Get(), inputMode) || !SetConsoleMode(outputHandle.Get(), outputMode))
            {
                SetConsoleMode(inputHandle.Get(), originalInputMode);
                SetConsoleMode(outputHandle.Get(), originalOutputMode);
                return std::nullopt;
            }

            const WindowsConsoleModeRestorer
                modeRestorer(inputHandle.Get(), originalInputMode, outputHandle.Get(), originalOutputMode);

            if (!WriteAll(outputHandle.Get(), request))
            {
                return std::nullopt;
            }

            std::string response;
            const auto deadline = std::chrono::steady_clock::now() + timeout;
            while (response.size() < k_MaximumResponseSize)
            {
                const int waitMilliseconds = WaitDurationMilliseconds(deadline, !response.empty());
                if (waitMilliseconds == 0)
                {
                    break;
                }

                const DWORD waitResult = WaitForSingleObject(inputHandle.Get(), static_cast<DWORD>(waitMilliseconds));
                if (waitResult == WAIT_TIMEOUT)
                {
                    break;
                }

                if (waitResult != WAIT_OBJECT_0)
                {
                    return std::nullopt;
                }

                if (!HasReadableCharacter(inputHandle.Get()))
                {
                    Sleep(1);
                    continue;
                }

                char buffer[1024];
                DWORD bytesRead = 0;
                if (!ReadFile(inputHandle.Get(), buffer, sizeof(buffer), &bytesRead, nullptr))
                {
                    return std::nullopt;
                }

                if (bytesRead > 0)
                {
                    response.append(buffer, bytesRead);
                }
            }

            return response;
        }
#else
        class PosixFileDescriptor
        {
          public:
            explicit PosixFileDescriptor(int descriptor)
                : m_Descriptor(descriptor)
            {
            }

            ~PosixFileDescriptor()
            {
                if (m_Descriptor >= 0)
                {
                    close(m_Descriptor);
                }
            }

            PosixFileDescriptor(const PosixFileDescriptor&) = delete;
            PosixFileDescriptor& operator=(const PosixFileDescriptor&) = delete;

            [[nodiscard]] int Get() const
            {
                return m_Descriptor;
            }

          private:
            int m_Descriptor = -1;
        };

        class PosixTerminalModeRestorer
        {
          public:
            PosixTerminalModeRestorer(int descriptor, const termios& mode)
                : m_Descriptor(descriptor)
                , m_Mode(mode)
            {
            }

            ~PosixTerminalModeRestorer()
            {
                tcsetattr(m_Descriptor, TCSANOW, &m_Mode);
            }

            PosixTerminalModeRestorer(const PosixTerminalModeRestorer&) = delete;
            PosixTerminalModeRestorer& operator=(const PosixTerminalModeRestorer&) = delete;

          private:
            int m_Descriptor;
            termios m_Mode;
        };

        [[nodiscard]] bool WriteAll(int descriptor, std::string_view request)
        {
            std::size_t offset = 0;
            while (offset < request.size())
            {
                const ssize_t written = write(descriptor, request.data() + offset, request.size() - offset);
                if (written > 0)
                {
                    offset += static_cast<std::size_t>(written);
                    continue;
                }

                if (written < 0 && errno == EINTR)
                {
                    continue;
                }

                return false;
            }

            return true;
        }

        [[nodiscard]] std::optional<std::string> QueryNativeTerminal(
            std::string_view request,
            std::chrono::milliseconds timeout
        )
        {
            PosixFileDescriptor terminal(open("/dev/tty", O_RDWR | O_NOCTTY | O_NONBLOCK | O_CLOEXEC));
            if (terminal.Get() < 0)
            {
                return std::nullopt;
            }

            termios originalMode { };
            if (tcgetattr(terminal.Get(), &originalMode) != 0)
            {
                return std::nullopt;
            }

            pollfd pendingInput { terminal.Get(), POLLIN, 0 };
            if (poll(&pendingInput, 1, 0) != 0)
            {
                return std::nullopt;
            }

            termios probeMode = originalMode;
            probeMode.c_lflag &= static_cast<tcflag_t>(~(ICANON | ECHO));
            probeMode.c_cc[VMIN] = 0;
            probeMode.c_cc[VTIME] = 0;
            if (tcsetattr(terminal.Get(), TCSANOW, &probeMode) != 0)
            {
                return std::nullopt;
            }

            const PosixTerminalModeRestorer modeRestorer(terminal.Get(), originalMode);

            if (!WriteAll(terminal.Get(), request))
            {
                return std::nullopt;
            }

            std::string response;
            const auto deadline = std::chrono::steady_clock::now() + timeout;
            while (response.size() < k_MaximumResponseSize)
            {
                const int waitMilliseconds = WaitDurationMilliseconds(deadline, !response.empty());
                if (waitMilliseconds == 0)
                {
                    break;
                }

                pollfd descriptor { terminal.Get(), POLLIN, 0 };
                const int pollResult = poll(&descriptor, 1, waitMilliseconds);
                if (pollResult == 0)
                {
                    break;
                }

                if (pollResult < 0)
                {
                    if (errno == EINTR)
                    {
                        continue;
                    }

                    return std::nullopt;
                }

                char buffer[1024];
                const ssize_t bytesRead = read(terminal.Get(), buffer, sizeof(buffer));
                if (bytesRead > 0)
                {
                    response.append(buffer, static_cast<std::size_t>(bytesRead));
                }
                else if (bytesRead < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)
                {
                    return std::nullopt;
                }
            }

            return response;
        }
#endif
    } // namespace
} // namespace FoxMediaTerm
