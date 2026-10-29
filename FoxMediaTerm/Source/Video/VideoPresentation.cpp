#include "VideoPresentation.h"

#include <chrono>
#include <expected>
#include <string>
#include <utility>

#include "TerminalOutput.h"

namespace FoxMediaTerm
{
    namespace
    {
        using Clock = std::chrono::steady_clock;

        class SynchronizedUpdateScope
        {
          public:
            SynchronizedUpdateScope()
            {
                const auto beginStart = Clock::now();
                m_BeginResult = TerminalOutput::WriteRaw(VideoPresentation::k_BeginSynchronizedUpdate, false);
                m_BeginDuration = Clock::now() - beginStart;
            }

            ~SynchronizedUpdateScope()
            {
                if (!m_Finished)
                {
                    VideoPresentation::EndSynchronizedOutputBestEffort();
                }
            }

            SynchronizedUpdateScope(const SynchronizedUpdateScope&) = delete;
            SynchronizedUpdateScope& operator=(const SynchronizedUpdateScope&) = delete;

            [[nodiscard]] const std::expected<void, std::string>& BeginResult() const noexcept
            {
                return m_BeginResult;
            }

            [[nodiscard]] Clock::duration BeginDuration() const noexcept
            {
                return m_BeginDuration;
            }

            [[nodiscard]] std::expected<void, std::string> Finish(Clock::duration& duration)
            {
                const auto endStart = Clock::now();
                auto result = TerminalOutput::WriteRaw(VideoPresentation::k_EndSynchronizedUpdate, true);
                duration = Clock::now() - endStart;
                m_Finished = true;

                return result;
            }

          private:
            std::expected<void, std::string> m_BeginResult;
            Clock::duration m_BeginDuration { };
            bool m_Finished = false;
        };

        [[nodiscard]] std::expected<void, std::string> WritePayload(
            std::string_view payload,
            bool usesAnsiUtf8,
            bool flush
        )
        {
            return usesAnsiUtf8 ? TerminalOutput::WriteAnsiUtf8(payload, flush)
                                : TerminalOutput::WriteRaw(payload, flush);
        }
    } // namespace

    EVideoPresentationMode VideoPresentation::Resolve(ETerminalProtocol protocol, EVideoPresentationMode requestedMode)
    {
        if (requestedMode != EVideoPresentationMode::SynchronizedOutput)
        {
            return EVideoPresentationMode::Immediate;
        }

        return protocol == ETerminalProtocol::ITerm2 || protocol == ETerminalProtocol::HalfBlockTrueColor
                   ? EVideoPresentationMode::SynchronizedOutput
                   : EVideoPresentationMode::Immediate;
    }

    VideoPresentationResult VideoPresentation::WriteFrame(
        std::string_view payload,
        bool usesAnsiUtf8,
        EVideoPresentationMode mode
    )
    {
        VideoPresentationResult result;
        if (mode == EVideoPresentationMode::Immediate)
        {
            const auto writeStart = Clock::now();
            auto written = WritePayload(payload, usesAnsiUtf8, true);
            result.frameWriteDuration = Clock::now() - writeStart;
            if (!written)
            {
                result.error = std::move(written.error());
            }

            return result;
        }

        SynchronizedUpdateScope synchronizedUpdate;
        result.syncBeginDuration = synchronizedUpdate.BeginDuration();
        if (synchronizedUpdate.BeginResult())
        {
            const auto writeStart = Clock::now();
            auto written = WritePayload(payload, usesAnsiUtf8, false);
            result.frameWriteDuration = Clock::now() - writeStart;
            if (!written)
            {
                result.error = std::move(written.error());
            }
        }
        else
        {
            result.error = synchronizedUpdate.BeginResult().error();
        }

        Clock::duration endDuration { };
        auto ended = synchronizedUpdate.Finish(endDuration);
        result.syncEndDuration = endDuration;
        if (!ended && result.error.empty())
        {
            result.error = std::move(ended.error());
        }

        return result;
    }

    void VideoPresentation::EndSynchronizedOutputBestEffort() noexcept
    {
        try
        {
            static_cast<void>(TerminalOutput::WriteRaw(k_EndSynchronizedUpdate, true));
        }
        catch (...)
        {
        }
    }
} // namespace FoxMediaTerm
