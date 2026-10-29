#include "VideoPlaybackClock.h"

namespace FoxMediaTerm
{
    VideoPlaybackClock::VideoPlaybackClock(std::chrono::microseconds mediaStart, Clock::time_point playbackStart)
        : m_MediaStart(mediaStart)
        , m_PlaybackStart(playbackStart)
    {
    }

    VideoPlaybackClock::Clock::time_point VideoPlaybackClock::Deadline(std::chrono::microseconds timestamp) const
    {
        return m_PlaybackStart + (timestamp - m_MediaStart);
    }

    bool VideoPlaybackClock::IsDue(std::chrono::microseconds timestamp, Clock::time_point now) const
    {
        return Deadline(timestamp) <= now;
    }

    std::chrono::microseconds VideoPlaybackClock::TimestampAt(Clock::time_point now) const
    {
        return m_MediaStart + std::chrono::duration_cast<std::chrono::microseconds>(now - m_PlaybackStart);
    }
} // namespace FoxMediaTerm
