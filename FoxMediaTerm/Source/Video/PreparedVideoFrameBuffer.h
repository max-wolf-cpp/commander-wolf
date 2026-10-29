#pragma once

#include <array>
#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <expected>
#include <mutex>
#include <optional>
#include <stop_token>
#include <string>

#include "TerminalSize.h"

namespace FoxMediaTerm
{
    class VideoPlaybackTelemetry;

    struct PreparedVideoFrame
    {
        std::chrono::microseconds timestamp { 0 };
        std::chrono::steady_clock::time_point readyTime;
        std::uint64_t index = 0;
        std::string payload;
        std::string alternatePayload;
        std::size_t kittyTransmitChunks = 0;
        std::size_t kittyDeleteSuffixBytes = 0;
        bool usesAnsiUtf8 = false;
        TerminalCellSize terminalSize;
    };

    enum class EPreparedFrameWaitStatus
    {
        Ready,
        Timeout,
        Finished,
    };

    struct PreparedFrameWaitResult
    {
        EPreparedFrameWaitStatus status = EPreparedFrameWaitStatus::Timeout;
        std::size_t slot = 0;
    };

    struct PreparedFrameSelection
    {
        std::size_t slot = 0;
        std::size_t dropped = 0;
    };

    class PreparedVideoFrameBuffer
    {
      public:
        static constexpr std::size_t k_SlotCount = 3;

        explicit PreparedVideoFrameBuffer(VideoPlaybackTelemetry* telemetry = nullptr);

        [[nodiscard]] std::optional<std::size_t> AcquirePreparationSlot(std::stop_token stopToken);
        void Publish(std::size_t slot, PreparedVideoFrame frame);
        void CancelPreparation(std::size_t slot);
        void FailPreparation(std::size_t slot, std::string error);
        void Finish();

        [[nodiscard]] std::expected<PreparedFrameWaitResult, std::string> WaitForReady(
            std::chrono::milliseconds timeout
        );
        [[nodiscard]] PreparedFrameSelection SelectNewestDue(
            std::size_t writingSlot,
            std::chrono::microseconds latestDueTimestamp
        );
        [[nodiscard]] const PreparedVideoFrame& Frame(std::size_t slot) const;
        [[nodiscard]] PreparedVideoFrame Take(std::size_t slot);
        void Release(std::size_t slot);

        [[nodiscard]] std::size_t ReadyCount() const;

      private:
        enum class ESlotState
        {
            Free,
            Preparing,
            Ready,
            Writing,
        };

        struct Slot
        {
            ESlotState state = ESlotState::Free;
            PreparedVideoFrame frame;
        };

        [[nodiscard]] std::optional<std::size_t> FindFreeSlot() const;
        [[nodiscard]] std::optional<std::size_t> FindOldestReadySlot() const;

        mutable std::mutex m_Mutex;
        std::condition_variable_any m_SlotAvailable;
        std::condition_variable m_FrameAvailable;
        std::array<Slot, k_SlotCount> m_Slots;
        std::optional<std::string> m_Error;
        VideoPlaybackTelemetry* m_Telemetry = nullptr;
        bool m_Finished = false;
    };
} // namespace FoxMediaTerm
