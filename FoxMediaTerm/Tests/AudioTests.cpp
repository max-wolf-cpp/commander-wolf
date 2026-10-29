#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <optional>
#include <ostream>
#include <string>
#include <system_error>
#include <thread>
#include <utility>

#include <gtest/gtest.h>

#include "AudioDecoder.h"
#include "AudioPcmRingBuffer.h"
#include "AudioPlayback.h"
#include "AudioVisualization.h"

namespace
{
    struct TemporaryFileCleanup
    {
        std::filesystem::path path;

        ~TemporaryFileCleanup()
        {
            std::error_code error;
            std::filesystem::remove(path, error);
        }
    };

    [[nodiscard]] bool WriteTestWaveFile(const std::filesystem::path& path);

    TEST(AudioPcmRingBuffer, PreservesInterleavedStereoFramesAcrossWraparound)
    {
        FoxMediaTerm::AudioPcmRingBuffer buffer(4);
        const std::array first { 1.0F, 10.0F, 2.0F, 20.0F, 3.0F, 30.0F };
        const std::array second { 4.0F, 40.0F, 5.0F, 50.0F, 6.0F, 60.0F };
        std::array<float, 4> initialOutput { };
        std::array<float, 8> wrappedOutput { };

        EXPECT_EQ(buffer.Write(first), 3);
        EXPECT_EQ(buffer.Read(initialOutput), 2);
        EXPECT_EQ(initialOutput, (std::array { 1.0F, 10.0F, 2.0F, 20.0F }));
        EXPECT_EQ(buffer.Write(second), 3);
        EXPECT_EQ(buffer.AvailableReadFrames(), 4);
        EXPECT_EQ(buffer.Read(wrappedOutput), 4);
        EXPECT_EQ(wrappedOutput, (std::array { 3.0F, 30.0F, 4.0F, 40.0F, 5.0F, 50.0F, 6.0F, 60.0F }));
        EXPECT_EQ(buffer.AvailableReadFrames(), 0);
        EXPECT_EQ(buffer.AvailableWriteFrames(), 4);
    }

    TEST(AudioDecoder, DecodesAndResamplesMonoPcmToStereoFloat48Khz)
    {
        const auto uniqueValue = std::chrono::steady_clock::now().time_since_epoch().count();
        TemporaryFileCleanup temporaryFile {
            std::filesystem::temp_directory_path()
                / ("FoxMediaTermAudioDecoder-" + std::to_string(uniqueValue) + ".wav"),
        };
        ASSERT_TRUE(WriteTestWaveFile(temporaryFile.path));

        auto opened = FoxMediaTerm::AudioDecoder::Open(temporaryFile.path);
        ASSERT_TRUE(opened.has_value()) << (opened ? "" : opened.error());
        ASSERT_TRUE(opened->has_value());
        FoxMediaTerm::AudioDecoder decoder = std::move(**opened);

        std::size_t totalFrames = 0;
        std::optional<std::chrono::microseconds> firstTimestamp;
        while (true)
        {
            auto chunk = decoder.ReadChunk();
            ASSERT_TRUE(chunk.has_value()) << (chunk ? "" : chunk.error());
            if (!*chunk)
            {
                break;
            }

            if (!firstTimestamp)
            {
                firstTimestamp = (**chunk).timestamp;
            }

            EXPECT_EQ((**chunk).samples.size() % FoxMediaTerm::AudioDecoder::k_ChannelCount, 0);
            for (std::size_t frame = 0; frame < (**chunk).FrameCount(); ++frame)
            {
                EXPECT_FLOAT_EQ((**chunk).samples[frame * 2], (**chunk).samples[frame * 2 + 1]);
            }

            totalFrames += (**chunk).FrameCount();
        }

        ASSERT_TRUE(firstTimestamp.has_value());
        EXPECT_EQ(*firstTimestamp, std::chrono::microseconds::zero());
        EXPECT_GE(totalFrames, 4'700);
        EXPECT_LE(totalFrames, 4'900);
    }

    TEST(AudioPlayback, DerivesTheMasterClockFromConsumedPcmFrames)
    {
        using namespace std::chrono_literals;

        EXPECT_EQ(FoxMediaTerm::AudioPlayback::MediaTimeForPlayedFrames(10s, 0), 10s);
        EXPECT_EQ(FoxMediaTerm::AudioPlayback::MediaTimeForPlayedFrames(10s, 24'000), 10'500ms);
        EXPECT_EQ(FoxMediaTerm::AudioPlayback::MediaTimeForPlayedFrames(10s, 48'000), 11s);
    }

    TEST(AudioVisualization, CalculatesBoundedPeakLevelsWithoutAllocationDependentState)
    {
        std::array<float, 128> samples { };
        samples[0] = -0.25F;
        samples[1] = 0.5F;
        samples[126] = 2.0F;

        const auto levels = FoxMediaTerm::AudioVisualization::CalculateLevels(samples, 2);

        EXPECT_FLOAT_EQ(levels.front(), 0.5F);
        EXPECT_FLOAT_EQ(levels.back(), 1.0F);
        EXPECT_TRUE(std::ranges::all_of(levels, [](float level) { return level >= 0.0F && level <= 1.0F; }));
        EXPECT_EQ(
            FoxMediaTerm::AudioVisualization::CalculateLevels({ }, 2),
            FoxMediaTerm::AudioVisualization::Levels { }
        );
    }

    TEST(AudioVisualization, PublishesCoherentSnapshotsAcrossThreads)
    {
        FoxMediaTerm::AudioVisualization visualization;
        std::array<float, FoxMediaTerm::AudioVisualization::k_LevelCount * 2> lowSamples { };
        std::array<float, FoxMediaTerm::AudioVisualization::k_LevelCount * 2> highSamples { };
        lowSamples.fill(0.25F);
        highSamples.fill(0.75F);
        visualization.Update(lowSamples, 2);

        std::atomic_bool writerFinished = false;
        std::jthread writer([&] {
            for (std::size_t iteration = 0; iteration < 10'000; ++iteration)
            {
                visualization.Update(iteration % 2 == 0 ? lowSamples : highSamples, 2);
            }
            writerFinished.store(true, std::memory_order_release);
        });

        bool snapshotsWereCoherent = true;
        do
        {
            const auto levels = visualization.Snapshot();
            snapshotsWereCoherent = snapshotsWereCoherent && std::ranges::all_of(levels, [&](float level) {
                                        return level == levels.front();
                                    });
        }
        while (!writerFinished.load(std::memory_order_acquire));
        writer.join();

        EXPECT_TRUE(snapshotsWereCoherent);
    }

    void WriteLittleEndian(std::ostream& output, std::uint32_t value, std::size_t byteCount)
    {
        for (std::size_t index = 0; index < byteCount; ++index)
        {
            output.put(static_cast<char>((value >> (index * 8)) & 0xFF));
        }
    }

    [[nodiscard]] bool WriteTestWaveFile(const std::filesystem::path& path)
    {
        constexpr std::uint32_t k_SampleRate = 8'000;
        constexpr std::uint16_t k_ChannelCount = 1;
        constexpr std::uint16_t k_BitsPerSample = 16;
        constexpr std::uint32_t k_SampleCount = 800;
        constexpr std::uint32_t k_DataSize = k_SampleCount * sizeof(std::int16_t);

        std::ofstream output(path, std::ios::binary);
        output.write("RIFF", 4);
        WriteLittleEndian(output, 36 + k_DataSize, 4);
        output.write("WAVEfmt ", 8);
        WriteLittleEndian(output, 16, 4);
        WriteLittleEndian(output, 1, 2);
        WriteLittleEndian(output, k_ChannelCount, 2);
        WriteLittleEndian(output, k_SampleRate, 4);
        WriteLittleEndian(output, k_SampleRate * k_ChannelCount * k_BitsPerSample / 8, 4);
        WriteLittleEndian(output, k_ChannelCount * k_BitsPerSample / 8, 2);
        WriteLittleEndian(output, k_BitsPerSample, 2);
        output.write("data", 4);
        WriteLittleEndian(output, k_DataSize, 4);

        for (std::uint32_t index = 0; index < k_SampleCount; ++index)
        {
            const std::int16_t sample = (index / 20) % 2 == 0 ? 12'000 : -12'000;
            WriteLittleEndian(output, static_cast<std::uint16_t>(sample), 2);
        }

        return output.good();
    }
} // namespace
