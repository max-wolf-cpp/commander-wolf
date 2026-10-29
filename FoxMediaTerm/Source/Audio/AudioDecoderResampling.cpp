#include <limits>
#include <utility>

extern "C"
{
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libswresample/swresample.h>
}

#include "AudioDecoderImplementation.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr int k_MicrosecondsPerSecond = 1'000'000;
    } // namespace

    std::expected<std::optional<AudioPcmChunk>, std::string> AudioDecoder::Implementation::ConvertFrame()
    {
        auto resamplerReady = EnsureResampler();
        if (!resamplerReady)
        {
            return std::unexpected(resamplerReady.error());
        }

        const int outputCapacity = swr_get_out_samples(resampleContext, decodedFrame->nb_samples);
        if (outputCapacity < 0
            || static_cast<std::size_t>(outputCapacity) > std::numeric_limits<std::size_t>::max() / 2)
        {
            return std::unexpected("The resampled audio chunk is too large");
        }

        AudioPcmChunk chunk {
            .timestamp = ReadTimestamp(),
            .samples = std::vector<float>(static_cast<std::size_t>(outputCapacity) * AudioDecoder::k_ChannelCount),
        };
        std::uint8_t* output[] = { reinterpret_cast<std::uint8_t*>(chunk.samples.data()), nullptr };
        const std::uint8_t** input = const_cast<const std::uint8_t**>(decodedFrame->extended_data);
        const int convertedFrames =
            swr_convert(resampleContext, output, outputCapacity, input, decodedFrame->nb_samples);
        if (convertedFrames < 0)
        {
            return std::unexpected(FfmpegError("Resampling an audio frame", convertedFrames));
        }

        chunk.samples.resize(static_cast<std::size_t>(convertedFrames) * AudioDecoder::k_ChannelCount);
        if (chunk.samples.empty())
        {
            return std::optional<AudioPcmChunk> { };
        }

        return std::optional<AudioPcmChunk>(std::move(chunk));
    }

    std::expected<std::optional<AudioPcmChunk>, std::string> AudioDecoder::Implementation::DrainResampler()
    {
        if (resampleContext == nullptr || resamplerDrained)
        {
            return std::optional<AudioPcmChunk> { };
        }

        const int outputCapacity = swr_get_out_samples(resampleContext, 0);
        if (outputCapacity <= 0)
        {
            resamplerDrained = true;
            return std::optional<AudioPcmChunk> { };
        }

        AudioPcmChunk chunk {
            .timestamp = fallbackTimestamp,
            .samples = std::vector<float>(static_cast<std::size_t>(outputCapacity) * AudioDecoder::k_ChannelCount),
        };
        std::uint8_t* output[] = { reinterpret_cast<std::uint8_t*>(chunk.samples.data()), nullptr };
        const int convertedFrames = swr_convert(resampleContext, output, outputCapacity, nullptr, 0);
        if (convertedFrames < 0)
        {
            return std::unexpected(FfmpegError("Draining the audio resampler", convertedFrames));
        }

        if (convertedFrames == 0)
        {
            resamplerDrained = true;
            return std::optional<AudioPcmChunk> { };
        }

        chunk.samples.resize(static_cast<std::size_t>(convertedFrames) * AudioDecoder::k_ChannelCount);
        fallbackTimestamp += std::chrono::microseconds(av_rescale_q(
            convertedFrames,
            AVRational { 1, AudioDecoder::k_SampleRate },
            AVRational { 1, k_MicrosecondsPerSecond }
        ));

        return std::optional<AudioPcmChunk>(std::move(chunk));
    }

    std::expected<void, std::string> AudioDecoder::Implementation::EnsureResampler()
    {
        const int sampleRate = decodedFrame->sample_rate > 0 ? decodedFrame->sample_rate : codecContext->sample_rate;
        if (sampleRate <= 0 || decodedFrame->format < 0)
        {
            return std::unexpected("The decoded audio frame has an invalid sample format or rate");
        }

        AVChannelLayout frameLayout { };
        if (decodedFrame->ch_layout.nb_channels > 0)
        {
            if (av_channel_layout_copy(&frameLayout, &decodedFrame->ch_layout) < 0)
            {
                return std::unexpected("Unable to copy the decoded audio channel layout");
            }
        }
        else
        {
            const int channels = codecContext->ch_layout.nb_channels > 0 ? codecContext->ch_layout.nb_channels : 2;
            av_channel_layout_default(&frameLayout, channels);
        }

        const AVSampleFormat sampleFormat = static_cast<AVSampleFormat>(decodedFrame->format);
        const bool unchanged = resampleContext != nullptr && inputSampleRate == sampleRate
                               && inputFormat == sampleFormat
                               && av_channel_layout_compare(&inputLayout, &frameLayout) == 0;
        if (unchanged)
        {
            av_channel_layout_uninit(&frameLayout);
            return { };
        }

        swr_free(&resampleContext);
        av_channel_layout_uninit(&inputLayout);
        if (av_channel_layout_copy(&inputLayout, &frameLayout) < 0)
        {
            av_channel_layout_uninit(&frameLayout);
            return std::unexpected("Unable to retain the decoded audio channel layout");
        }

        AVChannelLayout outputLayout = AV_CHANNEL_LAYOUT_STEREO;
        const int configureResult = swr_alloc_set_opts2(
            &resampleContext,
            &outputLayout,
            AV_SAMPLE_FMT_FLT,
            AudioDecoder::k_SampleRate,
            &frameLayout,
            sampleFormat,
            sampleRate,
            0,
            nullptr
        );
        av_channel_layout_uninit(&frameLayout);
        if (configureResult < 0)
        {
            return std::unexpected(FfmpegError("Configuring the audio resampler", configureResult));
        }

        const int initializeResult = swr_init(resampleContext);
        if (initializeResult < 0)
        {
            return std::unexpected(FfmpegError("Initializing the audio resampler", initializeResult));
        }

        inputSampleRate = sampleRate;
        inputFormat = sampleFormat;

        return { };
    }

    std::chrono::microseconds AudioDecoder::Implementation::ReadTimestamp()
    {
        std::chrono::microseconds timestamp = fallbackTimestamp;
        if (decodedFrame->best_effort_timestamp != AV_NOPTS_VALUE)
        {
            timestamp = RescaleTimestamp(decodedFrame->best_effort_timestamp, audioStream->time_base);
        }

        const int sampleRate = decodedFrame->sample_rate > 0 ? decodedFrame->sample_rate : inputSampleRate;
        if (sampleRate > 0)
        {
            fallbackTimestamp = timestamp
                                + std::chrono::microseconds(av_rescale_q(
                                    decodedFrame->nb_samples,
                                    AVRational { 1, sampleRate },
                                    AVRational { 1, k_MicrosecondsPerSecond }
                                ));
        }

        return timestamp;
    }

    std::chrono::microseconds AudioDecoder::Implementation::RescaleTimestamp(
        std::int64_t timestamp,
        AVRational timeBase
    )
    {
        return std::chrono::microseconds(av_rescale_q(timestamp, timeBase, AVRational { 1, k_MicrosecondsPerSecond }));
    }
} // namespace FoxMediaTerm
