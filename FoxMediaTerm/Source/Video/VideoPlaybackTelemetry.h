#pragma once

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <mutex>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "ImagePrepareMetrics.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"
#include "TerminalSize.h"
#include "VideoPresentation.h"

namespace FoxMediaTerm
{
    struct KittyLifecycleEvent
    {
        std::uint64_t frameIndex = 0;
        std::uint32_t imageId = 0;
        std::uint32_t placementId = 0;
        std::uint32_t deletedImageId = 0;
        std::size_t transmitChunks = 0;
    };

    struct VideoPlaybackTelemetrySnapshot
    {
        std::uint64_t framesDecoded = 0;
        std::uint64_t framesPrepared = 0;
        std::uint64_t framesWritten = 0;
        std::uint64_t dropBeforePrepare = 0;
        std::uint64_t dropLateAfterPrepare = 0;
        std::uint64_t dropStaleResize = 0;
        std::uint64_t dropSlotsFull = 0;
        std::vector<std::int64_t> decodeNanoseconds;
        std::vector<std::int64_t> demuxNanoseconds;
        std::vector<std::int64_t> codecNanoseconds;
        std::vector<std::int64_t> scaleNanoseconds;
        std::vector<std::int64_t> prepareNanoseconds;
        std::vector<std::int64_t> writeNanoseconds;
        std::vector<std::int64_t> syncBeginNanoseconds;
        std::vector<std::int64_t> syncEndNanoseconds;
        std::vector<std::uint64_t> payloadBytes;
        std::vector<std::int64_t> presentationLatenessNanoseconds;
        std::vector<std::int64_t> outputIntervalNanoseconds;
        std::vector<std::int64_t> prepareStartLatenessNanoseconds;
        std::vector<std::int64_t> readyLatenessNanoseconds;
        std::vector<std::int64_t> writeCompletionLatenessNanoseconds;
        std::vector<std::int64_t> outputReadyWaitNanoseconds;
        std::vector<std::int64_t> outputScheduleWaitNanoseconds;
        std::vector<std::int64_t> scheduleWakeOvershootNanoseconds;
        std::array<std::vector<std::int64_t>, static_cast<std::size_t>(EImagePrepareStage::Count)> prepareStages;
        std::vector<std::uint64_t> finalPayloadCapacities;
        std::vector<std::uint64_t> finalPayloadReallocations;
        std::vector<std::uint64_t> temporaryBufferCapacities;
        std::vector<std::uint64_t> temporaryBufferReallocations;
        std::uint64_t prepareInputPixels = 0;
        std::uint64_t prepareInputBytes = 0;
        std::size_t maxReadyDepth = 0;
        std::int64_t producerWaitTotalNanoseconds = 0;
        std::int64_t producerWaitMaxNanoseconds = 0;
        double videoFps = 0.0;
        double targetFrameIntervalMilliseconds = 0.0;
        int sourceWidth = 0;
        int sourceHeight = 0;
        std::string sourceFormat;
        int scaledWidth = 0;
        int scaledHeight = 0;
        std::string scaledFormat;
        std::uint64_t rawScaledBytes = 0;
        TerminalCellSize terminalSize;
        TerminalCellPixelSize cellPixelSize;
        std::vector<KittyLifecycleEvent> kittyLifecycleEvents;
        EVideoPresentationMode presentationMode = EVideoPresentationMode::Immediate;
        bool audioEnabled = false;
        int audioSampleRate = 0;
        int audioChannels = 0;
        std::string audioFormat;
        std::uint64_t audioDecodedPcmFrames = 0;
        std::uint64_t audioQueuedPcmFrames = 0;
        std::uint64_t audioPlayedPcmFrames = 0;
        std::uint64_t audioUnderrunFrames = 0;
        std::size_t audioBufferCapacityFrames = 0;
        bool audioDecoderFinished = false;
    };

    class VideoPlaybackTelemetry
    {
      public:
        using Duration = std::chrono::steady_clock::duration;

        void Reset();
        void RecordDecode(Duration total, Duration demux, Duration codec);
        void RecordScale(Duration duration);
        void RecordPrepared(
            const ImagePrepareMetrics& metrics,
            std::chrono::microseconds timestamp,
            std::chrono::steady_clock::time_point prepareStart,
            std::chrono::steady_clock::time_point readyTime
        );
        void RecordWrite(Duration duration, bool succeeded);
        void RecordSyncBeginOverhead(Duration duration);
        void RecordSyncEndOverhead(Duration duration);
        void RecordDropBeforePrepare();
        void RecordDropLateAfterPrepare(std::size_t count);
        void RecordDropStaleResize();
        void RecordDropSlotsFull();
        void RecordPresentationLateness(Duration duration);
        void RecordOutputInterval(Duration duration);
        void RecordWriteCompletionLateness(Duration duration);
        void RecordOutputReadyWait(Duration duration);
        void RecordOutputScheduleWait(Duration duration);
        void RecordScheduleWakeOvershoot(Duration duration);
        void ObserveReadyDepth(std::size_t depth);
        void RecordProducerWait(Duration duration);
        void SetPlaybackTimeline(
            std::chrono::microseconds mediaStart,
            std::chrono::steady_clock::time_point playbackStart
        );
        void RecordVideoRate(double fps, std::chrono::microseconds targetFrameInterval);
        void RecordSourceGeometry(int width, int height, std::string_view format);
        void RecordScaledGeometry(int width, int height, std::uint64_t rawBytes, std::string_view format);
        void RecordTerminalGeometry(TerminalCellSize terminalSize, TerminalCellPixelSize cellPixelSize);
        void RecordKittyLifecycle(
            std::uint64_t frameIndex,
            std::uint32_t imageId,
            std::uint32_t placementId,
            std::uint32_t deletedImageId,
            std::size_t transmitChunks
        );
        void SetPresentationMode(EVideoPresentationMode mode);
        void SetAudioConfiguration(bool enabled, int sampleRate, int channels, std::string_view format);
        void RecordAudioSummary(
            std::uint64_t decodedPcmFrames,
            std::uint64_t queuedPcmFrames,
            std::uint64_t playedPcmFrames,
            std::uint64_t underrunFrames,
            std::size_t bufferCapacityFrames,
            bool decoderFinished
        );

        [[nodiscard]] VideoPlaybackTelemetrySnapshot Snapshot() const;
        [[nodiscard]] std::string Format(ETerminalProtocol protocol) const;

      private:
        struct PendingPrepareWindow
        {
            std::chrono::microseconds timestamp { 0 };
            std::chrono::steady_clock::time_point start;
            std::chrono::steady_clock::time_point ready;
        };

        void RecordPrepareLateness(const PendingPrepareWindow& window);

        mutable std::mutex m_Mutex;
        VideoPlaybackTelemetrySnapshot m_Data;
        std::vector<PendingPrepareWindow> m_PendingPrepareWindows;
        std::optional<std::chrono::microseconds> m_MediaStart;
        std::optional<std::chrono::steady_clock::time_point> m_PlaybackStart;
    };
} // namespace FoxMediaTerm
