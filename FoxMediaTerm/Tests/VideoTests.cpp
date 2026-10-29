#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <span>
#include <stop_token>
#include <string>
#include <string_view>
#include <system_error>
#include <thread>
#include <vector>

#include <gtest/gtest.h>

#include "ImagePrepareMetrics.h"
#include "PreparedVideoFrameBuffer.h"
#include "VideoDecoder.h"
#include "VideoFrame.h"
#include "VideoPlaybackClock.h"
#include "VideoPlaybackTelemetry.h"
#include "VideoPngEncoder.h"
#include "VideoPresentation.h"
#include "VideoPreviewPlayback.h"
#include "VideoThumbnail.h"

namespace
{
    [[nodiscard]] std::uint32_t ReadPngUint32(const std::vector<std::uint8_t>& png, std::size_t offset)
    {
        return (static_cast<std::uint32_t>(png[offset]) << 24) | (static_cast<std::uint32_t>(png[offset + 1]) << 16)
               | (static_cast<std::uint32_t>(png[offset + 2]) << 8) | static_cast<std::uint32_t>(png[offset + 3]);
    }

    TEST(VideoDecoder, AcceptsMediaExtensionsWithoutConfusingDocumentsAndProjectFiles)
    {
        for (const char* path :
             { "clip.MP4", "clip.avi", "clip.y4m", "song.mp3", "song.flac", "voice.WAV", "picture.webp", "clip.nut" })
        {
            EXPECT_TRUE(FoxMediaTerm::VideoDecoder::HasSupportedExtension(path)) << path;
        }
        for (const char* path : { "solution.sln",
                                  "model.stl",
                                  "data.dat",
                                  "source.str",
                                  "subtitle.srt",
                                  "index.idx",
                                  "source.avs",
                                  "source.v",
                                  "picture.avif",
                                  "picture.heic",
                                  "notes.txt",
                                  "notes.md",
                                  "source.cpp" })
        {
            EXPECT_FALSE(FoxMediaTerm::VideoDecoder::HasSupportedExtension(path)) << path;
        }
    }

    [[nodiscard]] bool WriteTestY4mFile(const std::filesystem::path& path, std::span<const std::uint8_t> frameLuminance)
    {
        constexpr int k_Width = 4;
        constexpr int k_Height = 4;
        std::ofstream output(path, std::ios::binary);
        output << "YUV4MPEG2 W4 H4 F25:1 Ip A1:1 C420jpeg\n";
        for (const std::uint8_t luminance : frameLuminance)
        {
            output << "FRAME\n";
            const std::array<std::uint8_t, k_Width * k_Height> luma = [luminance] {
                std::array<std::uint8_t, k_Width * k_Height> value { };
                value.fill(luminance);
                return value;
            }();
            constexpr std::array<std::uint8_t, k_Width * k_Height / 4> k_Chroma { 128, 128, 128, 128 };
            output.write(reinterpret_cast<const char*>(luma.data()), luma.size());
            output.write(reinterpret_cast<const char*>(k_Chroma.data()), k_Chroma.size());
            output.write(reinterpret_cast<const char*>(k_Chroma.data()), k_Chroma.size());
        }
        return output.good();
    }

    struct TemporaryFileCleanup
    {
        std::filesystem::path path;

        ~TemporaryFileCleanup()
        {
            std::error_code error;
            std::filesystem::remove(path, error);
        }
    };

    TEST(VideoPresentation, EnablesDec2026OnlyForExplicitlySupportedBackends)
    {
        using FoxMediaTerm::ETerminalProtocol;
        using FoxMediaTerm::EVideoPresentationMode;

        EXPECT_EQ(
            FoxMediaTerm::VideoPresentation::Resolve(ETerminalProtocol::ITerm2, EVideoPresentationMode::Immediate),
            EVideoPresentationMode::Immediate
        );
        EXPECT_EQ(
            FoxMediaTerm::VideoPresentation::Resolve(
                ETerminalProtocol::ITerm2,
                EVideoPresentationMode::SynchronizedOutput
            ),
            EVideoPresentationMode::SynchronizedOutput
        );
        EXPECT_EQ(
            FoxMediaTerm::VideoPresentation::Resolve(
                ETerminalProtocol::HalfBlockTrueColor,
                EVideoPresentationMode::SynchronizedOutput
            ),
            EVideoPresentationMode::SynchronizedOutput
        );
        EXPECT_EQ(
            FoxMediaTerm::VideoPresentation::Resolve(
                ETerminalProtocol::Kitty,
                EVideoPresentationMode::SynchronizedOutput
            ),
            EVideoPresentationMode::Immediate
        );
        EXPECT_EQ(
            FoxMediaTerm::VideoPresentation::Resolve(
                ETerminalProtocol::Sixel,
                EVideoPresentationMode::SynchronizedOutput
            ),
            EVideoPresentationMode::Immediate
        );
        EXPECT_EQ(FoxMediaTerm::VideoPresentation::k_BeginSynchronizedUpdate, "\x1b[?2026h");
        EXPECT_EQ(FoxMediaTerm::VideoPresentation::k_EndSynchronizedUpdate, "\x1b[?2026l");
    }

    TEST(VideoPngEncoder, EncodesRgb24WithTheSelectedRealtimeBackend)
    {
        const FoxMediaTerm::RenderableImage image {
            .width = 2,
            .height = 1,
            .pixels = { 255, 0, 0, 0, 255, 0 },
            .pixelFormat = FoxMediaTerm::EImagePixelFormat::Rgb24,
        };
        FoxMediaTerm::ImagePrepareMetrics metrics;

        auto png = FoxMediaTerm::VideoPngEncoder::Encode(image, &metrics);

        ASSERT_TRUE(png.has_value()) << (png.has_value() ? "" : png.error());
        ASSERT_GE(png->size(), 24);
        constexpr std::array<std::uint8_t, 8> k_PngSignature { 137, 80, 78, 71, 13, 10, 26, 10 };
        EXPECT_TRUE(std::equal(k_PngSignature.begin(), k_PngSignature.end(), png->begin()));
        EXPECT_EQ(ReadPngUint32(*png, 16), 2U);
        EXPECT_EQ(ReadPngUint32(*png, 20), 1U);
        EXPECT_TRUE(metrics.stageRecorded[static_cast<std::size_t>(FoxMediaTerm::EImagePrepareStage::PngEncode)]);

        const FoxMediaTerm::PngEncoderConfiguration configuration = FoxMediaTerm::VideoPngEncoder::Configuration();
        EXPECT_TRUE(configuration.implementation == "fpnge default" || configuration.implementation == "fpng 1-pass");
        EXPECT_FALSE(configuration.cpuRequirement.empty());
    }

    TEST(VideoThumbnail, SkipsBlankOpeningFramesAndReturnsTheFirstMeaningfulFrame)
    {
        const auto uniqueValue = std::chrono::steady_clock::now().time_since_epoch().count();
        TemporaryFileCleanup temporaryFile {
            std::filesystem::temp_directory_path()
                / ("FoxMediaTermVideoThumbnail-" + std::to_string(uniqueValue) + ".y4m"),
        };
        constexpr std::array<std::uint8_t, 3> k_Luminance { 16, 16, 200 };
        ASSERT_TRUE(WriteTestY4mFile(temporaryFile.path, k_Luminance));
        auto decoder = FoxMediaTerm::VideoDecoder::Open(temporaryFile.path);
        ASSERT_TRUE(decoder.has_value()) << (decoder ? "" : decoder.error());

        auto thumbnail = FoxMediaTerm::VideoThumbnail::Read(*decoder, 4, 4);

        ASSERT_TRUE(thumbnail.has_value()) << (thumbnail ? "" : thumbnail.error());
        ASSERT_TRUE(thumbnail->has_value());
        EXPECT_TRUE(FoxMediaTerm::VideoThumbnail::IsMeaningful(**thumbnail));
        ASSERT_FALSE((**thumbnail).pixels.empty());
        EXPECT_GT((**thumbnail).pixels.front(), 150);
    }

    TEST(VideoPreviewPlayback, PreparesTimedFramesInTheBackgroundWithoutAudio)
    {
        const auto uniqueValue = std::chrono::steady_clock::now().time_since_epoch().count();
        TemporaryFileCleanup temporaryFile {
            std::filesystem::temp_directory_path()
                / ("FoxMediaTermVideoPreview-" + std::to_string(uniqueValue) + ".y4m"),
        };
        constexpr std::array<std::uint8_t, 2> k_Luminance { 100, 180 };
        ASSERT_TRUE(WriteTestY4mFile(temporaryFile.path, k_Luminance));

        constexpr FoxMediaTerm::TerminalCellSize k_TerminalSize { 4, 2 };
        auto playback = FoxMediaTerm::VideoPreviewPlayback::Start(
            temporaryFile.path,
            FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor,
            { 1, 1, true },
            { 0, 0, 4, 2 },
            k_TerminalSize,
            false
        );

        ASSERT_TRUE(playback.has_value()) << (playback ? "" : playback.error());
        EXPECT_FALSE((*playback)->HasAudio());
        EXPECT_FALSE((*playback)->IsMuted());
        (*playback)->SetMuted(true);
        EXPECT_TRUE((*playback)->IsMuted());
        std::size_t preparedFrameCount = 0;
        bool finished = false;
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds { 2 };
        while (!finished && std::chrono::steady_clock::now() < deadline)
        {
            auto update = (*playback)->Poll(k_TerminalSize);
            ASSERT_TRUE(update.has_value()) << (update ? "" : update.error());
            if (!update->payload.empty())
            {
                ++preparedFrameCount;
                EXPECT_TRUE(update->usesAnsiUtf8);
                EXPECT_NE(update->payload.find("\x1b["), std::string::npos);
            }
            finished = update->finished;
            if (!finished)
            {
                std::this_thread::sleep_for(std::chrono::milliseconds { 2 });
            }
        }
        EXPECT_TRUE(finished);
        EXPECT_GE(preparedFrameCount, 1U);
    }

    TEST(VideoPlaybackClock, DropsEveryLateFrameAndKeepsTheNewestCurrentFrame)
    {
        using namespace std::chrono_literals;

        const FoxMediaTerm::VideoPlaybackClock::Clock::time_point playbackStart { 5s };
        const FoxMediaTerm::VideoPlaybackClock playbackClock(10s, playbackStart);
        const std::array timestamps { 10'000ms, 10'033ms, 10'066ms, 10'100ms };
        const auto now = playbackStart + 70ms;

        std::size_t selectedFrame = 0;
        while (selectedFrame + 1 < timestamps.size() && playbackClock.IsDue(timestamps[selectedFrame + 1], now))
        {
            ++selectedFrame;
        }

        EXPECT_EQ(selectedFrame, 2);
        EXPECT_FALSE(playbackClock.IsDue(timestamps[3], now));
        EXPECT_EQ(playbackClock.TimestampAt(now), 10'070ms);
    }

    [[nodiscard]] FoxMediaTerm::PreparedVideoFrame PreparedFrame(std::uint64_t index)
    {
        return {
            .timestamp = std::chrono::milliseconds { static_cast<std::int64_t>(index * 33) },
            .index = index,
            .payload = std::to_string(index),
            .usesAnsiUtf8 = index % 2 == 0,
            .terminalSize = { 80, 24 },
        };
    }

    std::size_t PublishFrame(
        FoxMediaTerm::PreparedVideoFrameBuffer& buffer,
        std::stop_token stopToken,
        std::uint64_t index
    )
    {
        const auto slot = buffer.AcquirePreparationSlot(stopToken);
        EXPECT_TRUE(slot.has_value());
        if (!slot)
        {
            return 0;
        }
        buffer.Publish(*slot, PreparedFrame(index));
        return *slot;
    }

    TEST(PreparedVideoFrameBuffer, HoldsOneWritingAndTwoReadyPayloadsInThreeFixedSlots)
    {
        FoxMediaTerm::VideoPlaybackTelemetry telemetry;
        FoxMediaTerm::PreparedVideoFrameBuffer buffer(&telemetry);
        std::stop_source stopSource;

        PublishFrame(buffer, stopSource.get_token(), 0);
        auto first = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_TRUE(first.has_value());
        ASSERT_EQ(first->status, FoxMediaTerm::EPreparedFrameWaitStatus::Ready);

        PublishFrame(buffer, stopSource.get_token(), 1);
        PublishFrame(buffer, stopSource.get_token(), 2);

        EXPECT_EQ(FoxMediaTerm::PreparedVideoFrameBuffer::k_SlotCount, 3);
        EXPECT_EQ(buffer.ReadyCount(), 2);
        EXPECT_EQ(buffer.Frame(first->slot).payload, "0");
        EXPECT_EQ(telemetry.Snapshot().maxReadyDepth, 2);
        FoxMediaTerm::PreparedVideoFrame taken = buffer.Take(first->slot);
        EXPECT_EQ(taken.payload, "0");
        const auto recycledSlot = buffer.AcquirePreparationSlot(stopSource.get_token());
        ASSERT_TRUE(recycledSlot.has_value());
        buffer.CancelPreparation(*recycledSlot);
    }

    TEST(PreparedVideoFrameBuffer, WakesABlockedProducerWhenStopIsRequested)
    {
        FoxMediaTerm::PreparedVideoFrameBuffer buffer;
        std::stop_source fillStopSource;
        PublishFrame(buffer, fillStopSource.get_token(), 0);
        PublishFrame(buffer, fillStopSource.get_token(), 1);
        PublishFrame(buffer, fillStopSource.get_token(), 2);
        std::atomic<bool> acquiredSlot = true;

        std::jthread blockedProducer([&](std::stop_token stopToken) {
            acquiredSlot.store(buffer.AcquirePreparationSlot(stopToken).has_value());
        });
        blockedProducer.request_stop();
        blockedProducer.join();

        EXPECT_FALSE(acquiredSlot.load());
    }

    TEST(PreparedVideoFrameBuffer, DropsEveryDueFrameExceptTheNewestPreparedPayload)
    {
        FoxMediaTerm::PreparedVideoFrameBuffer buffer;
        std::stop_source stopSource;

        PublishFrame(buffer, stopSource.get_token(), 0);
        auto first = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_TRUE(first.has_value());
        ASSERT_EQ(first->status, FoxMediaTerm::EPreparedFrameWaitStatus::Ready);
        PublishFrame(buffer, stopSource.get_token(), 1);
        PublishFrame(buffer, stopSource.get_token(), 2);

        const FoxMediaTerm::PreparedFrameSelection selection =
            buffer.SelectNewestDue(first->slot, std::chrono::milliseconds { 70 });

        EXPECT_EQ(selection.dropped, 2);
        EXPECT_EQ(buffer.Frame(selection.slot).index, 2);
        EXPECT_EQ(buffer.Frame(selection.slot).payload, "2");
        EXPECT_EQ(buffer.ReadyCount(), 0);
        buffer.Release(selection.slot);
        buffer.Finish();

        auto finished = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_TRUE(finished.has_value());
        EXPECT_EQ(finished->status, FoxMediaTerm::EPreparedFrameWaitStatus::Finished);
    }

    TEST(PreparedVideoFrameBuffer, PreservesPreparedFramesThatAreNotDueYet)
    {
        FoxMediaTerm::PreparedVideoFrameBuffer buffer;
        std::stop_source stopSource;

        PublishFrame(buffer, stopSource.get_token(), 0);
        auto first = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_TRUE(first.has_value());
        ASSERT_EQ(first->status, FoxMediaTerm::EPreparedFrameWaitStatus::Ready);
        PublishFrame(buffer, stopSource.get_token(), 1);
        PublishFrame(buffer, stopSource.get_token(), 2);

        const FoxMediaTerm::PreparedFrameSelection selection =
            buffer.SelectNewestDue(first->slot, std::chrono::milliseconds { 40 });

        EXPECT_EQ(selection.dropped, 1);
        EXPECT_EQ(buffer.Frame(selection.slot).index, 1);
        EXPECT_EQ(buffer.ReadyCount(), 1);
        buffer.Release(selection.slot);

        auto future = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_TRUE(future.has_value());
        ASSERT_EQ(future->status, FoxMediaTerm::EPreparedFrameWaitStatus::Ready);
        EXPECT_EQ(buffer.Frame(future->slot).index, 2);
        buffer.Release(future->slot);
    }

    TEST(PreparedVideoFrameBuffer, PropagatesProducerErrorsToTheOutputThread)
    {
        FoxMediaTerm::PreparedVideoFrameBuffer buffer;
        std::stop_source stopSource;
        const auto slot = buffer.AcquirePreparationSlot(stopSource.get_token());
        ASSERT_TRUE(slot.has_value());

        buffer.FailPreparation(*slot, "prepare failed");

        auto ready = buffer.WaitForReady(std::chrono::milliseconds { 0 });
        ASSERT_FALSE(ready.has_value());
        EXPECT_EQ(ready.error(), "prepare failed");
    }

    TEST(VideoFrame, TreatsOnlyFramesWhoseWholeIntervalPassedAsEntirelyLate)
    {
        const FoxMediaTerm::VideoFrame frame {
            .image = { 1, 1, { 1, 2, 3, 4 } },
            .timestamp = std::chrono::milliseconds { 100 },
            .duration = std::chrono::milliseconds { 33 },
            .index = 1,
        };

        EXPECT_FALSE(FoxMediaTerm::IsEntirelyLate(frame, std::chrono::milliseconds { 132 }));
        EXPECT_TRUE(FoxMediaTerm::IsEntirelyLate(frame, std::chrono::milliseconds { 133 }));
    }

    TEST(VideoPlaybackTelemetry, ReportsCountersTailPercentilesAndProtocolAfterPlayback)
    {
        using namespace std::chrono_literals;

        FoxMediaTerm::VideoPlaybackTelemetry telemetry;
        for (int sample = 1; sample <= 100; ++sample)
        {
            telemetry.RecordDecode(std::chrono::milliseconds { sample }, 1ms, 2ms);
        }
        telemetry.RecordScale(2ms);
        const auto playbackStart = std::chrono::steady_clock::now();
        telemetry.SetPlaybackTimeline(0ms, playbackStart);
        FoxMediaTerm::ImagePrepareMetrics firstMetrics;
        firstMetrics.RecordStage(FoxMediaTerm::EImagePrepareStage::PrepareTotal, 3ms);
        firstMetrics.RecordStage(FoxMediaTerm::EImagePrepareStage::PngEncode, 2ms);
        firstMetrics.inputPixels = 500;
        firstMetrics.inputBytes = 2000;
        firstMetrics.finalPayloadBytes = 1000;
        firstMetrics.finalPayloadCapacity = 1200;
        firstMetrics.finalPayloadReallocations = 1;
        firstMetrics.RecordTemporaryBuffer(2200, 2);
        telemetry.RecordPrepared(firstMetrics, 0ms, playbackStart + 1ms, playbackStart + 3ms);
        FoxMediaTerm::ImagePrepareMetrics secondMetrics = firstMetrics;
        secondMetrics.RecordStage(FoxMediaTerm::EImagePrepareStage::PrepareTotal, 2ms);
        secondMetrics.finalPayloadBytes = 3000;
        secondMetrics.finalPayloadCapacity = 3200;
        telemetry.RecordPrepared(secondMetrics, 40ms, playbackStart + 41ms, playbackStart + 45ms);
        telemetry.RecordWrite(4ms, true);
        telemetry.RecordWrite(6ms, false);
        telemetry.RecordDropBeforePrepare();
        telemetry.RecordDropLateAfterPrepare(2);
        telemetry.RecordDropStaleResize();
        telemetry.RecordPresentationLateness(7ms);
        telemetry.RecordWriteCompletionLateness(11ms);
        telemetry.RecordOutputInterval(16ms);
        telemetry.RecordOutputReadyWait(8ms);
        telemetry.RecordOutputScheduleWait(12ms);
        telemetry.RecordScheduleWakeOvershoot(3ms);
        telemetry.RecordScheduleWakeOvershoot(5ms);
        telemetry.ObserveReadyDepth(2);
        telemetry.RecordProducerWait(9ms);
        telemetry.RecordVideoRate(25.0, 40ms);
        telemetry.RecordSourceGeometry(1920, 1080, "yuv420p");
        telemetry.RecordScaledGeometry(800, 450, 1'440'000, "rgba");
        telemetry.RecordTerminalGeometry({ 100, 30 }, { 8, 16, true });
        telemetry.RecordKittyLifecycle(10, 101, 201, 0, 3);
        telemetry.RecordKittyLifecycle(11, 102, 202, 101, 3);
        telemetry.SetAudioConfiguration(true, 48'000, 2, "float32 interleaved");
        telemetry.RecordAudioSummary(96'000, 95'000, 48'000, 32, 96'000, false);

        const FoxMediaTerm::VideoPlaybackTelemetrySnapshot snapshot = telemetry.Snapshot();
        EXPECT_EQ(snapshot.framesDecoded, 100);
        EXPECT_EQ(snapshot.framesPrepared, 2);
        EXPECT_EQ(snapshot.framesWritten, 1);
        EXPECT_EQ(snapshot.dropBeforePrepare, 1);
        EXPECT_EQ(snapshot.dropLateAfterPrepare, 2);
        EXPECT_EQ(snapshot.dropStaleResize, 1);
        EXPECT_EQ(snapshot.dropSlotsFull, 0);
        EXPECT_EQ(snapshot.maxReadyDepth, 2);
        EXPECT_TRUE(snapshot.audioEnabled);
        EXPECT_EQ(snapshot.audioPlayedPcmFrames, 48'000);
        EXPECT_EQ(snapshot.audioUnderrunFrames, 32);

        const std::string report = telemetry.Format(FoxMediaTerm::ETerminalProtocol::Kitty);
        EXPECT_NE(report.find("Protocol: kty (Kitty)"), std::string::npos);
        EXPECT_NE(report.find("Presentation mode"), std::string::npos);
        EXPECT_NE(report.find("immediate"), std::string::npos);
        EXPECT_NE(report.find("50.500 / 95.000 / 99.000 ms"), std::string::npos);
        EXPECT_NE(report.find("2000.0 / 3000.0 bytes"), std::string::npos);
        EXPECT_NE(report.find("Drop because slots full   0"), std::string::npos);
        EXPECT_NE(report.find("Source frame"), std::string::npos);
        EXPECT_NE(report.find("1920x1080 yuv420p"), std::string::npos);
        EXPECT_NE(report.find("PNG filter + DEFLATE + output total"), std::string::npos);
        EXPECT_NE(report.find("PNG implementation"), std::string::npos);
        EXPECT_NE(report.find("Final payload capacity"), std::string::npos);
        EXPECT_NE(report.find("Lateness at prepare start"), std::string::npos);
        EXPECT_NE(report.find("Output wait for Ready"), std::string::npos);
        EXPECT_NE(report.find("Schedule wake overshoot"), std::string::npos);
        EXPECT_NE(report.find("4.000 / 3.000 / 5.000 / 5.000 / 5.000 ms"), std::string::npos);
        EXPECT_NE(report.find("Kitty command lifecycle (actual written frames)"), std::string::npos);
        EXPECT_NE(report.find("action=transmit a=t i=101 chunks=3"), std::string::npos);
        EXPECT_NE(report.find("action=delete none (initial frame)"), std::string::npos);
        EXPECT_NE(report.find("action=delete a=d d=I i=101"), std::string::npos);
        EXPECT_NE(report.find("Producer wait"), std::string::npos);
        EXPECT_NE(report.find("Playback clock"), std::string::npos);
        EXPECT_NE(report.find("audio device"), std::string::npos);
        EXPECT_NE(report.find("Audio underrun"), std::string::npos);
    }

    TEST(VideoPlaybackTelemetry, ReportsSynchronizedPresentationOverheadSeparatelyFromFrameWrite)
    {
        using namespace std::chrono_literals;

        FoxMediaTerm::VideoPlaybackTelemetry telemetry;
        telemetry.SetPresentationMode(FoxMediaTerm::EVideoPresentationMode::SynchronizedOutput);
        telemetry.RecordWrite(5ms, true);
        telemetry.RecordSyncBeginOverhead(1ms);
        telemetry.RecordSyncEndOverhead(2ms);

        const std::string report = telemetry.Format(FoxMediaTerm::ETerminalProtocol::ITerm2);

        EXPECT_NE(report.find("synchronized-2026"), std::string::npos);
        EXPECT_NE(report.find("Write                                  5.000 / 5.000 / 5.000 ms"), std::string::npos);
        EXPECT_NE(report.find("Sync begin overhead                    1.000 / 1.000 / 1.000 ms"), std::string::npos);
        EXPECT_NE(report.find("Sync end overhead                      2.000 / 2.000 / 2.000 ms"), std::string::npos);
    }

} // namespace
