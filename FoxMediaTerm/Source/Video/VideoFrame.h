#pragma once

#include <chrono>
#include <cstdint>

#include "Image.h"

namespace FoxMediaTerm
{
    struct VideoFrame
    {
        RasterImage image;
        std::chrono::microseconds timestamp { 0 };
        std::chrono::microseconds duration { 0 };
        std::uint64_t index = 0;
    };

    [[nodiscard]] inline bool IsEntirelyLate(const VideoFrame& frame, std::chrono::microseconds mediaTime)
    {
        return frame.duration > std::chrono::microseconds::zero() && frame.timestamp + frame.duration <= mediaTime;
    }
} // namespace FoxMediaTerm
