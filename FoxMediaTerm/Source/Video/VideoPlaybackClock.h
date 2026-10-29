#pragma once

#include <chrono>

namespace FoxMediaTerm
{
    class VideoPlaybackClock
    {
      public:
        using Clock = std::chrono::steady_clock;

        VideoPlaybackClock(std::chrono::microseconds mediaStart, Clock::time_point playbackStart);

        [[nodiscard]] Clock::time_point Deadline(std::chrono::microseconds timestamp) const;
        [[nodiscard]] bool IsDue(std::chrono::microseconds timestamp, Clock::time_point now) const;
        [[nodiscard]] std::chrono::microseconds TimestampAt(Clock::time_point now) const;

      private:
        std::chrono::microseconds m_MediaStart;
        Clock::time_point m_PlaybackStart;
    };
} // namespace FoxMediaTerm
