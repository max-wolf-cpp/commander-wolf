#pragma once

#include <atomic>
#include <mutex>
#include <span>
#include <stop_token>
#include <thread>

#include <miniaudio.h>

#include "AudioDecoder.h"
#include "AudioPcmRingBuffer.h"
#include "AudioPlayback.h"

namespace FoxMediaTerm
{
    struct AudioPlayback::Implementation
    {
        explicit Implementation(AudioDecoder selectedDecoder, std::chrono::microseconds selectedMediaStart);
        ~Implementation();

        [[nodiscard]] std::expected<void, std::string> Prime(const AudioPcmChunk& firstChunk);
        [[nodiscard]] std::expected<void, std::string> StartDevice();
        void Run(std::stop_token stopToken);

        static void DataCallback(ma_device* callbackDevice, void* output, const void* input, ma_uint32 frameCount);
        [[nodiscard]] bool QueueChunk(const AudioPcmChunk& chunk, std::stop_token stopToken = { });
        [[nodiscard]] bool WriteSilence(std::uint64_t frameCount, std::stop_token stopToken = { });
        [[nodiscard]] bool WriteFrames(std::span<const float> samples, std::stop_token stopToken = { });
        void SetError(std::string message);

        AudioPcmRingBuffer ring;
        AudioVisualization visualization;
        AudioDecoder decoder;
        ma_device device { };
        std::jthread producer;
        std::chrono::microseconds mediaStart { 0 };
        AudioPlayback::Clock::time_point playbackStart;
        std::uint64_t timelineFrame = 0;
        std::atomic<std::uint64_t> decodedPcmFrames { 0 };
        std::atomic<std::uint64_t> queuedPcmFrames { 0 };
        std::atomic<std::uint64_t> playedFrames { 0 };
        std::atomic<std::uint64_t> underrunFrames { 0 };
        std::atomic<bool> decoderFinished { false };
        std::atomic<bool> muted { false };
        mutable std::mutex errorMutex;
        std::optional<std::string> error;
        bool deviceInitialized = false;
    };
} // namespace FoxMediaTerm
