#include <algorithm>
#include <cmath>
#include <iomanip>
#include <numeric>
#include <sstream>
#include <string_view>

#include "SixelRenderer.h"
#include "VideoPlaybackTelemetry.h"
#include "VideoPngEncoder.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr double k_NanosecondsPerMillisecond = 1'000'000.0;

        struct Distribution
        {
            double average = 0.0;
            double p50 = 0.0;
            double p95 = 0.0;
            double p99 = 0.0;
            double maximum = 0.0;
            double total = 0.0;
            std::size_t count = 0;
            bool hasSamples = false;
        };

        void AppendConfiguration(
            std::ostringstream& report,
            const VideoPlaybackTelemetrySnapshot& data,
            ETerminalProtocol protocol
        );
        void AppendKittyLifecycle(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
        void AppendGeometry(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
        void AppendFrameTimings(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
        void AppendPayloadStatistics(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
        void AppendThroughput(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
        void AppendScheduling(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data);
    } // namespace

    std::string VideoPlaybackTelemetry::Format(ETerminalProtocol protocol) const
    {
        const VideoPlaybackTelemetrySnapshot data = Snapshot();
        std::ostringstream report;

        AppendConfiguration(report, data, protocol);
        AppendKittyLifecycle(report, data);
        AppendGeometry(report, data);
        AppendFrameTimings(report, data);
        AppendPayloadStatistics(report, data);
        AppendThroughput(report, data);
        AppendScheduling(report, data);

        return report.str();
    }

    namespace
    {
        template <typename TValue> [[nodiscard]] double Percentile(const std::vector<TValue>& sorted, double percentile)
        {
            const double rank = std::ceil(percentile * static_cast<double>(sorted.size()));
            const std::size_t index = static_cast<std::size_t>(std::max(1.0, rank)) - 1;

            return static_cast<double>(sorted[index]);
        }

        template <typename TValue>
        [[nodiscard]] Distribution Summarize(const std::vector<TValue>& samples, double divisor)
        {
            if (samples.empty())
            {
                return { };
            }

            std::vector<TValue> sorted = samples;
            std::ranges::sort(sorted);
            const long double total = std::accumulate(
                sorted.begin(),
                sorted.end(),
                static_cast<long double>(0),
                [](long double sum, TValue value) { return sum + static_cast<long double>(value); }
            );

            return {
                .average = static_cast<double>(total / static_cast<long double>(sorted.size()) / divisor),
                .p50 = Percentile(sorted, 0.50) / divisor,
                .p95 = Percentile(sorted, 0.95) / divisor,
                .p99 = Percentile(sorted, 0.99) / divisor,
                .maximum = static_cast<double>(sorted.back()) / divisor,
                .total = static_cast<double>(total / divisor),
                .count = sorted.size(),
                .hasSamples = true,
            };
        }

        [[nodiscard]] constexpr std::string_view ProtocolName(ETerminalProtocol protocol)
        {
            switch (protocol)
            {
                case ETerminalProtocol::Auto:
                    return "auto";
                case ETerminalProtocol::ITerm2:
                    return "it2 (iTerm2)";
                case ETerminalProtocol::Sixel:
                    return "sxl (Sixel)";
                case ETerminalProtocol::Kitty:
                    return "kty (Kitty)";
                case ETerminalProtocol::HalfBlockTrueColor:
                    return "hb-tc (half-block true color)";
                case ETerminalProtocol::HalfBlock256:
                    return "hb-256 (half-block 256 colors)";
                case ETerminalProtocol::AsciiGrayscale:
                    return "ag (ASCII grayscale)";
            }

            return "unknown";
        }

        void AppendTiming(std::ostringstream& report, std::string_view name, const Distribution& distribution)
        {
            report << std::left << std::setw(39) << name;
            if (!distribution.hasSamples)
            {
                report << "n/a\n";
                return;
            }

            report << std::right << std::fixed << std::setprecision(3) << distribution.average << " / "
                   << distribution.p95 << " / " << distribution.p99 << " ms\n";
        }

        void AppendWait(std::ostringstream& report, std::string_view name, const Distribution& distribution)
        {
            report << std::left << std::setw(39) << name;
            if (!distribution.hasSamples)
            {
                report << "n/a\n";
                return;
            }

            report << std::right << std::fixed << std::setprecision(3) << distribution.total << " / "
                   << distribution.average << " / " << distribution.maximum << " ms / " << distribution.count
                   << " (total / avg / max / count)\n";
        }

        void AppendOvershoot(std::ostringstream& report, const Distribution& distribution)
        {
            report << std::left << std::setw(39) << "Schedule wake overshoot";
            if (!distribution.hasSamples)
            {
                report << "n/a\n";
                return;
            }

            report << std::right << std::fixed << std::setprecision(3) << distribution.average << " / "
                   << distribution.p50 << " / " << distribution.p95 << " / " << distribution.p99 << " / "
                   << distribution.maximum << " ms (avg / p50 / p95 / p99 / max)\n";
        }

        void AppendConfiguration(
            std::ostringstream& report,
            const VideoPlaybackTelemetrySnapshot& data,
            ETerminalProtocol protocol
        )
        {
            report << "WolfVideo playback statistics\nProtocol: " << ProtocolName(protocol) << "\n\n"
                   << std::left << std::setw(39) << "Build configuration" << FOX_MEDIA_TERM_BUILD_CONFIGURATION << '\n'
                   << std::left << std::setw(39) << "Project optimization flag" << FOX_MEDIA_TERM_OPTIMIZATION_FLAG
                   << '\n'
                   << std::left << std::setw(39) << "Presentation mode"
                   << VideoPresentationModeName(data.presentationMode) << '\n'
                   << std::left << std::setw(39) << "Playback clock"
                   << (data.audioEnabled ? "audio device" : "steady clock (no audio stream)") << '\n';
            if (data.audioEnabled)
            {
                report << std::left << std::setw(39) << "Audio PCM" << data.audioSampleRate << " Hz, "
                       << data.audioChannels << " channels, " << data.audioFormat << '\n'
                       << std::left << std::setw(39) << "Audio ring capacity" << data.audioBufferCapacityFrames
                       << " frames\n"
                       << std::left << std::setw(39) << "Audio PCM decoded" << data.audioDecodedPcmFrames << " frames\n"
                       << std::left << std::setw(39) << "Audio PCM queued" << data.audioQueuedPcmFrames << " frames\n"
                       << std::left << std::setw(39) << "Audio PCM played" << data.audioPlayedPcmFrames << " frames\n"
                       << std::left << std::setw(39) << "Audio underrun" << data.audioUnderrunFrames << " frames\n"
                       << std::left << std::setw(39) << "Audio decoder finished"
                       << (data.audioDecoderFinished ? "yes" : "no") << '\n';
            }
            report << '\n';

            if (protocol == ETerminalProtocol::Kitty || protocol == ETerminalProtocol::ITerm2)
            {
                const PngEncoderConfiguration png = VideoPngEncoder::Configuration();
                report << std::left << std::setw(39) << "PNG implementation" << png.implementation << '\n'
                       << std::left << std::setw(39) << "PNG compression level" << png.compressionLevel << '\n'
                       << std::left << std::setw(39) << "PNG filter"
                       << (png.forcedFilter < 0 ? "adaptive per row (filters 0..4)" : std::to_string(png.forcedFilter))
                       << '\n'
                       << std::left << std::setw(39) << "PNG compression strategy" << png.compressionStrategy << '\n'
                       << std::left << std::setw(39) << "PNG encoder context reuse"
                       << (png.reusesContext ? "yes" : "no") << '\n'
                       << std::left << std::setw(39) << "PNG CPU requirement" << png.cpuRequirement << '\n'
                       << std::left << std::setw(39) << "PNG filter / deflate timers"
                       << (png.exposesFilterAndDeflateTimings ? "available" : "not exposed by public API") << "\n\n";
            }
            else if (protocol == ETerminalProtocol::Sixel)
            {
                const SixelEncoderConfiguration sixel = SixelRenderer::Configuration();
                report << std::left << std::setw(39) << "Sixel implementation" << sixel.implementation << '\n'
                       << std::left << std::setw(39) << "Sixel palette colors" << sixel.paletteColors << '\n'
                       << std::left << std::setw(39) << "Sixel largest dimension" << sixel.largestDimensionAlgorithm
                       << '\n'
                       << std::left << std::setw(39) << "Sixel representative color"
                       << sixel.representativeColorAlgorithm << '\n'
                       << std::left << std::setw(39) << "Sixel quality" << sixel.qualityMode << '\n'
                       << std::left << std::setw(39) << "Sixel diffusion" << sixel.diffusionAlgorithm << '\n'
                       << std::left << std::setw(39) << "Sixel encoder context reuse"
                       << (sixel.reusesContext ? "yes" : "no") << '\n'
                       << std::left << std::setw(39) << "Sixel dither / packing timers"
                       << (sixel.exposesDitherAndPackingTimings ? "available" : "not exposed by public API") << "\n\n";
            }
        }

        void AppendKittyLifecycle(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            if (data.kittyLifecycleEvents.empty())
            {
                return;
            }

            report << "Kitty command lifecycle (actual written frames)\n";
            for (const KittyLifecycleEvent& event : data.kittyLifecycleEvents)
            {
                report << "frame " << event.frameIndex << ": image=" << event.imageId
                       << " placement=" << event.placementId << "\n"
                       << "  1. action=transmit a=t i=" << event.imageId << " chunks=" << event.transmitChunks << "\n"
                       << "  2. action=display/place a=p i=" << event.imageId << " p=" << event.placementId << "\n";
                if (event.deletedImageId != 0)
                {
                    report << "  3. action=delete a=d d=I i=" << event.deletedImageId << "\n";
                }
                else
                {
                    report << "  3. action=delete none (initial frame)\n";
                }
            }
            report << '\n';
        }

        void AppendGeometry(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            report << std::fixed << std::setprecision(3) << std::left << std::setw(39) << "Video FPS" << data.videoFps
                   << '\n'
                   << std::left << std::setw(39) << "Target frame interval" << data.targetFrameIntervalMilliseconds
                   << " ms\n"
                   << std::left << std::setw(39) << "Source frame" << data.sourceWidth << 'x' << data.sourceHeight
                   << ' ' << (data.sourceFormat.empty() ? "unknown" : data.sourceFormat) << '\n'
                   << std::left << std::setw(39) << "Scaled / Prepare input" << data.scaledWidth << 'x'
                   << data.scaledHeight << ' ' << (data.scaledFormat.empty() ? "unknown" : data.scaledFormat) << '\n'
                   << std::left << std::setw(39) << "Raw scaled frame" << data.rawScaledBytes << " bytes\n"
                   << std::left << std::setw(39) << "Terminal" << data.terminalSize.columns << 'x'
                   << data.terminalSize.rows << " cells\n"
                   << std::left << std::setw(39) << "Cell size";
            if (data.cellPixelSize.exact)
            {
                report << data.cellPixelSize.width << 'x' << data.cellPixelSize.height << " px\n\n";
            }
            else
            {
                report << "unknown (using " << data.cellPixelSize.width << 'x' << data.cellPixelSize.height
                       << " px estimate)\n\n";
            }
        }

        void AppendFrameTimings(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            report << "Frames decoded            " << data.framesDecoded << '\n'
                   << "Frames prepared           " << data.framesPrepared << '\n'
                   << "Frames written            " << data.framesWritten << "\n\n"
                   << "Drop before prepare       " << data.dropBeforePrepare << '\n'
                   << "Drop late after prepare   " << data.dropLateAfterPrepare << '\n'
                   << "Drop stale resize         " << data.dropStaleResize << '\n'
                   << "Drop because slots full   " << data.dropSlotsFull << "\n\n"
                   << "Timing                                  avg / p95 / p99\n";

            AppendTiming(report, "Read / demux", Summarize(data.demuxNanoseconds, k_NanosecondsPerMillisecond));
            AppendTiming(
                report,
                "Codec submit / receive",
                Summarize(data.codecNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(report, "Decode frame total", Summarize(data.decodeNanoseconds, k_NanosecondsPerMillisecond));
            AppendTiming(report, "Scale", Summarize(data.scaleNanoseconds, k_NanosecondsPerMillisecond));
            AppendTiming(report, "Prepare total", Summarize(data.prepareNanoseconds, k_NanosecondsPerMillisecond));
            for (std::size_t stage = 0; stage < static_cast<std::size_t>(EImagePrepareStage::Count); ++stage)
            {
                const EImagePrepareStage stageValue = static_cast<EImagePrepareStage>(stage);
                if (stageValue == EImagePrepareStage::PrepareTotal || data.prepareStages[stage].empty())
                {
                    continue;
                }

                AppendTiming(
                    report,
                    ImagePrepareStageName(stageValue),
                    Summarize(data.prepareStages[stage], k_NanosecondsPerMillisecond)
                );
            }

            AppendTiming(report, "Write", Summarize(data.writeNanoseconds, k_NanosecondsPerMillisecond));
            AppendTiming(
                report,
                "Sync begin overhead",
                Summarize(data.syncBeginNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(report, "Sync end overhead", Summarize(data.syncEndNanoseconds, k_NanosecondsPerMillisecond));
        }

        void AppendPayloadStatistics(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            const Distribution payload = Summarize(data.payloadBytes, 1.0);
            report << '\n' << std::left << std::setw(39) << "Final payload bytes";
            if (payload.hasSamples)
            {
                report << std::right << std::fixed << std::setprecision(1) << payload.average << " / " << payload.p95
                       << " bytes (avg / p95)\n";
            }
            else
            {
                report << "n/a\n";
            }

            const Distribution payloadCapacity = Summarize(data.finalPayloadCapacities, 1.0);
            report << std::left << std::setw(39) << "Final payload capacity";
            if (payloadCapacity.hasSamples)
            {
                report << std::right << std::fixed << std::setprecision(1) << payloadCapacity.average << " / "
                       << payloadCapacity.maximum << " bytes (avg / max)\n";
            }
            else
            {
                report << "n/a\n";
            }

            const Distribution payloadReallocations = Summarize(data.finalPayloadReallocations, 1.0);
            report << std::left << std::setw(39) << "Final payload reallocations";
            if (payloadReallocations.hasSamples)
            {
                report << std::right << std::fixed << std::setprecision(2) << payloadReallocations.average << " / "
                       << payloadReallocations.maximum << " (avg / max)\n";
            }
            else
            {
                report << "n/a\n";
            }

            const Distribution temporaryCapacity = Summarize(data.temporaryBufferCapacities, 1.0);
            report << std::left << std::setw(39) << "Tracked temporary main capacity";
            if (temporaryCapacity.hasSamples)
            {
                report << std::right << std::fixed << std::setprecision(1) << temporaryCapacity.average << " / "
                       << temporaryCapacity.maximum << " bytes (avg / max)\n";
            }
            else
            {
                report << "n/a\n";
            }

            const Distribution temporaryReallocations = Summarize(data.temporaryBufferReallocations, 1.0);
            report << std::left << std::setw(39) << "Tracked temporary main reallocations";
            if (temporaryReallocations.hasSamples)
            {
                report << std::right << std::fixed << std::setprecision(2) << temporaryReallocations.average << " / "
                       << temporaryReallocations.maximum << " (avg / max)\n";
            }
            else
            {
                report << "n/a\n";
            }
        }

        void AppendThroughput(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            const long double prepareNanoseconds = std::accumulate(
                data.prepareNanoseconds.begin(),
                data.prepareNanoseconds.end(),
                static_cast<long double>(0)
            );
            const double prepareSeconds = static_cast<double>(prepareNanoseconds / 1'000'000'000.0L);
            const std::uint64_t payloadTotal =
                std::accumulate(data.payloadBytes.begin(), data.payloadBytes.end(), std::uint64_t { 0 });
            const double prepareMegapixelsPerSecond =
                prepareSeconds > 0.0 ? static_cast<double>(data.prepareInputPixels) / prepareSeconds / 1'000'000.0
                                     : 0.0;
            const double rawMegabytesPerSecond =
                prepareSeconds > 0.0 ? static_cast<double>(data.prepareInputBytes) / prepareSeconds / 1'000'000.0 : 0.0;
            const double payloadMegabytesPerSecond =
                prepareSeconds > 0.0 ? static_cast<double>(payloadTotal) / prepareSeconds / 1'000'000.0 : 0.0;

            report << "\nThroughput\n"
                   << std::left << std::setw(39) << "Prepare throughput" << std::right << std::fixed
                   << std::setprecision(1) << prepareMegapixelsPerSecond << " MPix/s\n"
                   << std::left << std::setw(39) << "Raw input throughput" << std::right << rawMegabytesPerSecond
                   << " MB/s\n"
                   << std::left << std::setw(39) << "Payload generation" << std::right << payloadMegabytesPerSecond
                   << " MB/s\n\n";
        }

        void AppendScheduling(std::ostringstream& report, const VideoPlaybackTelemetrySnapshot& data)
        {
            AppendTiming(
                report,
                "Lateness at prepare start",
                Summarize(data.prepareStartLatenessNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(
                report,
                "Lateness when Ready",
                Summarize(data.readyLatenessNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(
                report,
                "Lateness at write start",
                Summarize(data.presentationLatenessNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(
                report,
                "Write completion lateness",
                Summarize(data.writeCompletionLatenessNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendTiming(
                report,
                "Output frame interval",
                Summarize(data.outputIntervalNanoseconds, k_NanosecondsPerMillisecond)
            );
            report << '\n';
            AppendWait(
                report,
                "Output wait for Ready",
                Summarize(data.outputReadyWaitNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendWait(
                report,
                "Output schedule wait",
                Summarize(data.outputScheduleWaitNanoseconds, k_NanosecondsPerMillisecond)
            );
            AppendOvershoot(report, Summarize(data.scheduleWakeOvershootNanoseconds, k_NanosecondsPerMillisecond));
            report << std::left << std::setw(39) << "Max Ready depth" << data.maxReadyDepth << '\n'
                   << std::left << std::setw(39) << "Producer wait" << std::right << std::fixed << std::setprecision(3)
                   << static_cast<double>(data.producerWaitTotalNanoseconds) / k_NanosecondsPerMillisecond << " / "
                   << static_cast<double>(data.producerWaitMaxNanoseconds) / k_NanosecondsPerMillisecond
                   << " ms (total / max)\n";
        }
    } // namespace
} // namespace FoxMediaTerm
