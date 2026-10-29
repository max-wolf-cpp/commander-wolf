#include "VideoPreviewPlayback.h"

#include <atomic>
#include <chrono>
#include <cstdint>
#include <limits>
#include <memory>
#include <optional>
#include <stop_token>
#include <thread>
#include <utility>

#include "AudioPlayback.h"
#include "ImagePrepareMetrics.h"
#include "KittyRenderer.h"
#include "PreparedVideoFrameBuffer.h"
#include "VideoDecoder.h"
#include "VideoFrame.h"
#include "VideoPlaybackClock.h"
#include "VideoPngEncoder.h"

namespace FoxMediaTerm
{
    struct VideoPreviewPlayback::Implementation
    {
        Implementation(
            ETerminalProtocol selectedProtocol,
            TerminalCellPixelSize selectedCellPixelSize,
            ImageCellRegion selectedRegion,
            TerminalCellSize selectedTerminalSize,
            std::unique_ptr<AudioPlayback> selectedAudio,
            bool selectedMuted
        )
            : protocol(selectedProtocol)
            , cellPixelSize(selectedCellPixelSize)
            , region(selectedRegion)
            , terminalSize(selectedTerminalSize)
            , audio(std::move(selectedAudio))
            , latestMediaTime(audio ? audio->CurrentMediaTime().count() : std::numeric_limits<std::int64_t>::min())
            , muted(selectedMuted)
        {
            if (audio)
            {
                audio->SetMuted(muted);
            }
        }

        ~Implementation()
        {
            if (producer.joinable())
            {
                producer.request_stop();
                producer.join();
            }
        }

        static void PrepareFrames(
            const std::filesystem::path& path,
            Implementation& playback,
            std::stop_token stopToken
        );

        ETerminalProtocol protocol;
        TerminalCellPixelSize cellPixelSize;
        ImageCellRegion region;
        TerminalCellSize terminalSize;
        PreparedVideoFrameBuffer buffer;
        std::unique_ptr<AudioPlayback> audio;
        std::optional<VideoPlaybackClock> playbackClock;
        std::optional<std::size_t> pendingSlot;
        std::optional<std::uint32_t> visibleKittyImageId;
        std::atomic<std::int64_t> latestMediaTime;
        bool muted = false;
        std::jthread producer;
    };

    VideoPreviewPlayback::VideoPreviewPlayback(std::unique_ptr<Implementation> implementation)
        : m_Implementation(std::move(implementation))
    {
    }

    VideoPreviewPlayback::~VideoPreviewPlayback() = default;

    std::expected<std::unique_ptr<VideoPreviewPlayback>, std::string> VideoPreviewPlayback::Start(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region,
        TerminalCellSize terminalSize,
        bool muted
    )
    {
        if (region.columns <= 0 || region.rows <= 0)
        {
            return std::unexpected("The video preview region is not ready");
        }

        auto audio = AudioPlayback::Open(path);
        if (!audio)
        {
            return std::unexpected(audio.error());
        }

        auto playback = std::unique_ptr<VideoPreviewPlayback>(new VideoPreviewPlayback(
            std::make_unique<Implementation>(protocol, cellPixelSize, region, terminalSize, std::move(*audio), muted)
        ));
        Implementation* implementation = playback->m_Implementation.get();
        implementation->producer = std::jthread([implementation, path](std::stop_token stopToken) {
            Implementation::PrepareFrames(path, *implementation, stopToken);
        });

        return playback;
    }

    std::expected<VideoPreviewUpdate, std::string> VideoPreviewPlayback::Poll(TerminalCellSize terminalSize)
    {
        Implementation& playback = *m_Implementation;
        if (terminalSize.columns != playback.terminalSize.columns || terminalSize.rows != playback.terminalSize.rows)
        {
            return VideoPreviewUpdate { .finished = true };
        }

        if (playback.audio)
        {
            if (const auto error = playback.audio->Error())
            {
                return std::unexpected("Audio playback failed: " + *error);
            }
        }

        if (!playback.pendingSlot)
        {
            auto ready = playback.buffer.WaitForReady(std::chrono::milliseconds { 0 });
            if (!ready)
            {
                return std::unexpected(ready.error());
            }

            if (ready->status == EPreparedFrameWaitStatus::Timeout)
            {
                return VideoPreviewUpdate { };
            }

            if (ready->status == EPreparedFrameWaitStatus::Finished)
            {
                return VideoPreviewUpdate { .finished = true };
            }

            playback.pendingSlot = ready->slot;
            if (!playback.audio && !playback.playbackClock)
            {
                const PreparedVideoFrame& firstFrame = playback.buffer.Frame(*playback.pendingSlot);
                playback.playbackClock.emplace(firstFrame.timestamp, VideoPlaybackClock::Clock::now());
            }
        }

        const auto now = VideoPlaybackClock::Clock::now();
        const std::chrono::microseconds mediaTime =
            playback.audio ? playback.audio->CurrentMediaTime() : playback.playbackClock->TimestampAt(now);
        playback.latestMediaTime.store(mediaTime.count(), std::memory_order_relaxed);
        if (playback.buffer.Frame(*playback.pendingSlot).timestamp > mediaTime)
        {
            return VideoPreviewUpdate { };
        }

        const PreparedFrameSelection selection = playback.buffer.SelectNewestDue(*playback.pendingSlot, mediaTime);
        playback.pendingSlot.reset();
        PreparedVideoFrame frame = playback.buffer.Take(selection.slot);
        std::string payload = std::move(frame.payload);

        // Only frames returned to the caller advance the visible Kitty ID; dropped frames do not.
        if (playback.protocol == ETerminalProtocol::Kitty && !frame.alternatePayload.empty())
        {
            if (playback.visibleKittyImageId == KittyRenderer::k_FirstImageId)
            {
                payload = std::move(frame.alternatePayload);
                playback.visibleKittyImageId = KittyRenderer::k_SecondImageId;
            }
            else
            {
                playback.visibleKittyImageId = KittyRenderer::k_FirstImageId;
            }
        }

        return VideoPreviewUpdate {
            .payload = std::move(payload),
            .usesAnsiUtf8 = frame.usesAnsiUtf8,
        };
    }

    void VideoPreviewPlayback::SetMuted(bool muted) noexcept
    {
        m_Implementation->muted = muted;
        if (m_Implementation->audio)
        {
            m_Implementation->audio->SetMuted(muted);
        }
    }

    bool VideoPreviewPlayback::IsMuted() const noexcept
    {
        return m_Implementation->muted;
    }

    bool VideoPreviewPlayback::HasAudio() const noexcept
    {
        return m_Implementation->audio != nullptr;
    }

    void VideoPreviewPlayback::Implementation::PrepareFrames(
        const std::filesystem::path& path,
        Implementation& playback,
        std::stop_token stopToken
    )
    {
        auto slot = playback.buffer.AcquirePreparationSlot(stopToken);
        if (!slot)
        {
            return;
        }

        auto decoderResult = VideoDecoder::Open(path);
        if (!decoderResult)
        {
            playback.buffer.FailPreparation(*slot, decoderResult.error());
            return;
        }

        VideoDecoder decoder = std::move(*decoderResult);
        const ImagePixelSize maximumSize =
            ImageRegionRenderer::MaximumPixelSize(playback.protocol, playback.cellPixelSize, playback.region);

        while (!stopToken.stop_requested())
        {
            auto decoded = decoder.ReadFrame(maximumSize.width, maximumSize.height);
            if (!decoded)
            {
                playback.buffer.FailPreparation(*slot, decoded.error());
                return;
            }

            if (!*decoded)
            {
                playback.buffer.CancelPreparation(*slot);
                playback.buffer.Finish();
                return;
            }

            if (stopToken.stop_requested())
            {
                playback.buffer.CancelPreparation(*slot);
                return;
            }

            VideoFrame frame = std::move(**decoded);
            const std::int64_t mediaTimeValue = playback.latestMediaTime.load(std::memory_order_relaxed);
            if (mediaTimeValue != std::numeric_limits<std::int64_t>::min()
                && IsEntirelyLate(frame, std::chrono::microseconds { mediaTimeValue }))
            {
                // Reuse this preparation slot while skipping late frames.
                continue;
            }

            const std::chrono::microseconds timestamp = frame.timestamp;
            const std::uint64_t index = frame.index;
            ImagePrepareMetrics metrics;
            auto view = ImageRegionRenderer::Prepare(
                std::move(frame.image),
                playback.protocol,
                playback.cellPixelSize,
                playback.region,
                &VideoPngEncoder::Encode,
                &metrics,
                EKittyPreparationMode::PingPongVideo
            );
            if (!view)
            {
                playback.buffer.FailPreparation(*slot, view.error());
                return;
            }

            if (stopToken.stop_requested())
            {
                playback.buffer.CancelPreparation(*slot);
                return;
            }

            playback.buffer.Publish(
                *slot,
                PreparedVideoFrame {
                    .timestamp = timestamp,
                    .readyTime = std::chrono::steady_clock::now(),
                    .index = index,
                    .payload = std::move(view->payload),
                    .alternatePayload = std::move(view->alternatePayload),
                    .kittyTransmitChunks = view->kittyTransmitChunks,
                    .kittyDeleteSuffixBytes = view->kittyDeleteSuffixBytes,
                    .usesAnsiUtf8 = view->usesAnsiUtf8,
                    .terminalSize = playback.terminalSize,
                }
            );
            slot = playback.buffer.AcquirePreparationSlot(stopToken);
            if (!slot)
            {
                return;
            }
        }

        playback.buffer.CancelPreparation(*slot);
    }
} // namespace FoxMediaTerm
