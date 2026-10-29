#include "VideoPlaybackTelemetry.h"

#include <algorithm>

namespace FoxMediaTerm
{
    void VideoPlaybackTelemetry::Reset()
    {
        std::lock_guard lock(m_Mutex);
        m_Data = { };
        m_PendingPrepareWindows.clear();
        m_MediaStart.reset();
        m_PlaybackStart.reset();
    }

    void VideoPlaybackTelemetry::RecordDecode(Duration total, Duration demux, Duration codec)
    {
        std::lock_guard lock(m_Mutex);
        ++m_Data.framesDecoded;
        m_Data.decodeNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(total).count());
        m_Data.demuxNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(demux).count());
        m_Data.codecNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(codec).count());
    }

    void VideoPlaybackTelemetry::RecordScale(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.scaleNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count());
    }

    void VideoPlaybackTelemetry::RecordPrepared(
        const ImagePrepareMetrics& metrics,
        std::chrono::microseconds timestamp,
        std::chrono::steady_clock::time_point prepareStart,
        std::chrono::steady_clock::time_point readyTime
    )
    {
        std::lock_guard lock(m_Mutex);
        ++m_Data.framesPrepared;
        const std::size_t prepareTotal = static_cast<std::size_t>(EImagePrepareStage::PrepareTotal);
        if (metrics.stageRecorded[prepareTotal])
        {
            m_Data.prepareNanoseconds.push_back(metrics.stageNanoseconds[prepareTotal]);
        }

        for (std::size_t stage = 0; stage < static_cast<std::size_t>(EImagePrepareStage::Count); ++stage)
        {
            if (metrics.stageRecorded[stage])
            {
                m_Data.prepareStages[stage].push_back(metrics.stageNanoseconds[stage]);
            }
        }

        m_Data.payloadBytes.push_back(metrics.finalPayloadBytes);
        m_Data.finalPayloadCapacities.push_back(metrics.finalPayloadCapacity);
        m_Data.finalPayloadReallocations.push_back(metrics.finalPayloadReallocations);
        m_Data.temporaryBufferCapacities.push_back(metrics.temporaryBufferCapacity);
        m_Data.temporaryBufferReallocations.push_back(metrics.temporaryBufferReallocations);
        m_Data.prepareInputPixels += metrics.inputPixels;
        m_Data.prepareInputBytes += metrics.inputBytes;

        const PendingPrepareWindow window {
            .timestamp = timestamp,
            .start = prepareStart,
            .ready = readyTime,
        };
        if (m_MediaStart && m_PlaybackStart)
        {
            RecordPrepareLateness(window);
        }
        else
        {
            m_PendingPrepareWindows.push_back(window);
        }
    }

    void VideoPlaybackTelemetry::RecordWrite(Duration duration, bool succeeded)
    {
        std::lock_guard lock(m_Mutex);
        if (succeeded)
        {
            ++m_Data.framesWritten;
        }

        m_Data.writeNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count());
    }

    void VideoPlaybackTelemetry::RecordSyncBeginOverhead(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.syncBeginNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count());
    }

    void VideoPlaybackTelemetry::RecordSyncEndOverhead(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.syncEndNanoseconds.push_back(std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count());
    }

    void VideoPlaybackTelemetry::RecordDropBeforePrepare()
    {
        std::lock_guard lock(m_Mutex);
        ++m_Data.dropBeforePrepare;
    }

    void VideoPlaybackTelemetry::RecordDropLateAfterPrepare(std::size_t count)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.dropLateAfterPrepare += count;
    }

    void VideoPlaybackTelemetry::RecordDropStaleResize()
    {
        std::lock_guard lock(m_Mutex);
        ++m_Data.dropStaleResize;
    }

    void VideoPlaybackTelemetry::RecordDropSlotsFull()
    {
        std::lock_guard lock(m_Mutex);
        ++m_Data.dropSlotsFull;
    }

    void VideoPlaybackTelemetry::RecordPresentationLateness(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.presentationLatenessNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::RecordOutputInterval(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.outputIntervalNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::RecordWriteCompletionLateness(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.writeCompletionLatenessNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::RecordOutputReadyWait(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.outputReadyWaitNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::RecordOutputScheduleWait(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.outputScheduleWaitNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::RecordScheduleWakeOvershoot(Duration duration)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.scheduleWakeOvershootNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count()
        );
    }

    void VideoPlaybackTelemetry::ObserveReadyDepth(std::size_t depth)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.maxReadyDepth = std::max(m_Data.maxReadyDepth, depth);
    }

    void VideoPlaybackTelemetry::RecordProducerWait(Duration duration)
    {
        const std::int64_t nanoseconds = std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();

        std::lock_guard lock(m_Mutex);
        m_Data.producerWaitTotalNanoseconds += nanoseconds;
        m_Data.producerWaitMaxNanoseconds = std::max(m_Data.producerWaitMaxNanoseconds, nanoseconds);
    }

    void VideoPlaybackTelemetry::SetPlaybackTimeline(
        std::chrono::microseconds mediaStart,
        std::chrono::steady_clock::time_point playbackStart
    )
    {
        std::lock_guard lock(m_Mutex);
        m_MediaStart = mediaStart;
        m_PlaybackStart = playbackStart;
        for (const PendingPrepareWindow& window : m_PendingPrepareWindows)
        {
            RecordPrepareLateness(window);
        }

        m_PendingPrepareWindows.clear();
    }

    void VideoPlaybackTelemetry::RecordVideoRate(double fps, std::chrono::microseconds targetFrameInterval)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.videoFps = fps;
        m_Data.targetFrameIntervalMilliseconds = std::chrono::duration<double, std::milli>(targetFrameInterval).count();
    }

    void VideoPlaybackTelemetry::RecordSourceGeometry(int width, int height, std::string_view format)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.sourceWidth = width;
        m_Data.sourceHeight = height;
        m_Data.sourceFormat = format;
    }

    void VideoPlaybackTelemetry::RecordScaledGeometry(
        int width,
        int height,
        std::uint64_t rawBytes,
        std::string_view format
    )
    {
        std::lock_guard lock(m_Mutex);
        m_Data.scaledWidth = width;
        m_Data.scaledHeight = height;
        m_Data.rawScaledBytes = rawBytes;
        m_Data.scaledFormat = format;
    }

    void VideoPlaybackTelemetry::RecordTerminalGeometry(
        TerminalCellSize terminalSize,
        TerminalCellPixelSize cellPixelSize
    )
    {
        std::lock_guard lock(m_Mutex);
        m_Data.terminalSize = terminalSize;
        m_Data.cellPixelSize = cellPixelSize;
    }

    void VideoPlaybackTelemetry::RecordKittyLifecycle(
        std::uint64_t frameIndex,
        std::uint32_t imageId,
        std::uint32_t placementId,
        std::uint32_t deletedImageId,
        std::size_t transmitChunks
    )
    {
        constexpr std::size_t k_MaxLoggedFrames = 6;
        std::lock_guard lock(m_Mutex);
        if (m_Data.kittyLifecycleEvents.size() >= k_MaxLoggedFrames)
        {
            return;
        }

        m_Data.kittyLifecycleEvents.push_back({ frameIndex, imageId, placementId, deletedImageId, transmitChunks });
    }

    void VideoPlaybackTelemetry::SetPresentationMode(EVideoPresentationMode mode)
    {
        std::lock_guard lock(m_Mutex);
        m_Data.presentationMode = mode;
    }

    void VideoPlaybackTelemetry::SetAudioConfiguration(
        bool enabled,
        int sampleRate,
        int channels,
        std::string_view format
    )
    {
        std::lock_guard lock(m_Mutex);
        m_Data.audioEnabled = enabled;
        m_Data.audioSampleRate = sampleRate;
        m_Data.audioChannels = channels;
        m_Data.audioFormat = format;
    }

    void VideoPlaybackTelemetry::RecordAudioSummary(
        std::uint64_t decodedPcmFrames,
        std::uint64_t queuedPcmFrames,
        std::uint64_t playedPcmFrames,
        std::uint64_t underrunFrames,
        std::size_t bufferCapacityFrames,
        bool decoderFinished
    )
    {
        std::lock_guard lock(m_Mutex);
        m_Data.audioDecodedPcmFrames = decodedPcmFrames;
        m_Data.audioQueuedPcmFrames = queuedPcmFrames;
        m_Data.audioPlayedPcmFrames = playedPcmFrames;
        m_Data.audioUnderrunFrames = underrunFrames;
        m_Data.audioBufferCapacityFrames = bufferCapacityFrames;
        m_Data.audioDecoderFinished = decoderFinished;
    }

    VideoPlaybackTelemetrySnapshot VideoPlaybackTelemetry::Snapshot() const
    {
        std::lock_guard lock(m_Mutex);
        return m_Data;
    }

    void VideoPlaybackTelemetry::RecordPrepareLateness(const PendingPrepareWindow& window)
    {
        const auto deadline = *m_PlaybackStart + (window.timestamp - *m_MediaStart);
        m_Data.prepareStartLatenessNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(window.start - deadline).count()
        );
        m_Data.readyLatenessNanoseconds.push_back(
            std::chrono::duration_cast<std::chrono::nanoseconds>(window.ready - deadline).count()
        );
    }
} // namespace FoxMediaTerm
