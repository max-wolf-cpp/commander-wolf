#pragma once

#include <chrono>
#include <optional>
#include <string>
#include <string_view>

#include "TerminalProtocol.h"

namespace FoxMediaTerm
{
    enum class EVideoPresentationMode
    {
        Immediate,
        SynchronizedOutput,
    };

    [[nodiscard]] constexpr std::string_view VideoPresentationModeName(EVideoPresentationMode mode)
    {
        return mode == EVideoPresentationMode::SynchronizedOutput ? "synchronized-2026" : "immediate";
    }

    struct VideoPresentationResult
    {
        std::chrono::steady_clock::duration frameWriteDuration { };
        std::optional<std::chrono::steady_clock::duration> syncBeginDuration;
        std::optional<std::chrono::steady_clock::duration> syncEndDuration;
        std::string error;

        [[nodiscard]] bool Succeeded() const noexcept
        {
            return error.empty();
        }
    };

    class VideoPresentation
    {
      public:
        static constexpr std::string_view k_BeginSynchronizedUpdate = "\x1b[?2026h";
        static constexpr std::string_view k_EndSynchronizedUpdate = "\x1b[?2026l";

        [[nodiscard]] static EVideoPresentationMode Resolve(
            ETerminalProtocol protocol,
            EVideoPresentationMode requestedMode
        );
        [[nodiscard]] static VideoPresentationResult WriteFrame(
            std::string_view payload,
            bool usesAnsiUtf8,
            EVideoPresentationMode mode
        );
        static void EndSynchronizedOutputBestEffort() noexcept;
    };
} // namespace FoxMediaTerm
