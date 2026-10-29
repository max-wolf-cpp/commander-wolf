#include <algorithm>
#include <utility>

#include "AudioPlaybackImplementation.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::size_t k_BufferSeconds = 2;
    } // namespace

    AudioPlayback::Implementation::Implementation(
        AudioDecoder selectedDecoder,
        std::chrono::microseconds selectedMediaStart
    )
        : ring(static_cast<std::size_t>(AudioPlayback::k_SampleRate) * k_BufferSeconds)
        , decoder(std::move(selectedDecoder))
        , mediaStart(selectedMediaStart)
    {
    }

    AudioPlayback::Implementation::~Implementation()
    {
        if (producer.joinable())
        {
            producer.request_stop();
            producer.join();
        }

        if (deviceInitialized)
        {
            ma_device_uninit(&device);
        }
    }

    std::expected<void, std::string> AudioPlayback::Implementation::StartDevice()
    {
        ma_device_config config = ma_device_config_init(ma_device_type_playback);
        config.playback.format = ma_format_f32;
        config.playback.channels = AudioPlayback::k_ChannelCount;
        config.sampleRate = AudioPlayback::k_SampleRate;
        config.dataCallback = &DataCallback;
        config.pUserData = this;

        const ma_result initializeResult = ma_device_init(nullptr, &config, &device);
        if (initializeResult != MA_SUCCESS)
        {
            return std::unexpected(
                std::string("Initializing the audio device failed: ") + ma_result_description(initializeResult)
            );
        }

        deviceInitialized = true;

        playbackStart = AudioPlayback::Clock::now();
        const ma_result startResult = ma_device_start(&device);
        if (startResult != MA_SUCCESS)
        {
            return std::unexpected(
                std::string("Starting the audio device failed: ") + ma_result_description(startResult)
            );
        }

        if (!decoderFinished.load(std::memory_order_acquire))
        {
            producer = std::jthread([this](std::stop_token stopToken) { Run(stopToken); });
        }

        return { };
    }

    void AudioPlayback::Implementation::DataCallback(
        ma_device* callbackDevice,
        void* output,
        const void*,
        ma_uint32 frameCount
    )
    {
        auto* self = static_cast<Implementation*>(callbackDevice->pUserData);
        auto* samples = static_cast<float*>(output);
        const std::size_t sampleCount = static_cast<std::size_t>(frameCount) * AudioPcmRingBuffer::k_ChannelCount;

        std::fill_n(samples, sampleCount, 0.0F);

        const std::size_t readFrames = self->ring.Read(std::span<float>(samples, sampleCount));
        self->visualization.Update(
            std::span<const float>(samples, readFrames * AudioPcmRingBuffer::k_ChannelCount),
            AudioPcmRingBuffer::k_ChannelCount
        );

        if (self->muted.load(std::memory_order_relaxed))
        {
            std::fill_n(samples, sampleCount, 0.0F);
        }

        self->playedFrames.fetch_add(frameCount, std::memory_order_relaxed);
        self->underrunFrames.fetch_add(frameCount - readFrames, std::memory_order_relaxed);
    }
} // namespace FoxMediaTerm
