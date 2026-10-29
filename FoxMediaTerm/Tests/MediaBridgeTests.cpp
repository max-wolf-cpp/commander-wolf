#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <span>
#include <string>
#include <string_view>
#include <thread>
#include <vector>

#include <FoxMediaTerm/FoxMediaTerm.h>
#include <gtest/gtest.h>

#include "VideoDecoder.h"

namespace
{
    TEST(TerminalQuery, RejectsInvalidArgumentsWithoutTouchingTheTerminal)
    {
        FoxMediaTermBuffer response { };
        FoxMediaTermError error { };
        EXPECT_EQ(FoxMediaTermQueryTerminal(nullptr, 0, 10, &response, &error), FOX_MEDIA_TERM_STATUS_ERROR);
        EXPECT_NE(error.data, nullptr);
        EXPECT_EQ(response.context, nullptr);
        FoxMediaTermReleaseError(&error);
    }

    [[nodiscard]] std::string Utf8Path(const std::filesystem::path& path)
    {
        const std::u8string text = path.generic_u8string();
        return std::string(reinterpret_cast<const char*>(text.data()), text.size());
    }

    void WriteY4m(const std::filesystem::path& path, std::span<const std::uint8_t> luminanceValues)
    {
        constexpr int k_Width = 4;
        constexpr int k_Height = 4;
        std::ofstream output(path, std::ios::binary);
        ASSERT_TRUE(output);
        output << "YUV4MPEG2 W4 H4 F25:1 Ip A1:1 C420jpeg\n";
        for (const std::uint8_t luminance : luminanceValues)
        {
            output << "FRAME\n";
            std::array<std::uint8_t, k_Width * k_Height> luma { };
            luma.fill(luminance);
            constexpr std::array<std::uint8_t, k_Width * k_Height / 4> k_Chroma { 128, 128, 128, 128 };
            output.write(reinterpret_cast<const char*>(luma.data()), luma.size());
            output.write(reinterpret_cast<const char*>(k_Chroma.data()), k_Chroma.size());
            output.write(reinterpret_cast<const char*>(k_Chroma.data()), k_Chroma.size());
        }
        ASSERT_TRUE(output.good());
    }

    void WriteLittleEndian(std::ostream& output, std::uint32_t value, std::size_t byteCount)
    {
        for (std::size_t index = 0; index < byteCount; ++index)
        {
            output.put(static_cast<char>((value >> (index * 8)) & 0xFFU));
        }
    }

    void WriteWave(const std::filesystem::path& path)
    {
        constexpr std::uint32_t k_SampleRate = 8'000;
        constexpr std::uint32_t k_SampleCount = 800;
        constexpr std::uint32_t k_DataSize = k_SampleCount * sizeof(std::int16_t);
        std::ofstream output(path, std::ios::binary);
        ASSERT_TRUE(output);
        output.write("RIFF", 4);
        WriteLittleEndian(output, 36 + k_DataSize, 4);
        output.write("WAVEfmt ", 8);
        WriteLittleEndian(output, 16, 4);
        WriteLittleEndian(output, 1, 2);
        WriteLittleEndian(output, 1, 2);
        WriteLittleEndian(output, k_SampleRate, 4);
        WriteLittleEndian(output, k_SampleRate * 2, 4);
        WriteLittleEndian(output, 2, 2);
        WriteLittleEndian(output, 16, 2);
        output.write("data", 4);
        WriteLittleEndian(output, k_DataSize, 4);
        for (std::uint32_t index = 0; index < k_SampleCount; ++index)
        {
            WriteLittleEndian(output, index % 2 == 0 ? 12'000U : static_cast<std::uint16_t>(-12'000), 2);
        }
        ASSERT_TRUE(output.good());
    }

    void WriteBase64(const std::filesystem::path& path, std::string_view encoded)
    {
        const auto decode = [](char value) -> int {
            if (value >= 'A' && value <= 'Z')
            {
                return value - 'A';
            }
            if (value >= 'a' && value <= 'z')
            {
                return value - 'a' + 26;
            }
            if (value >= '0' && value <= '9')
            {
                return value - '0' + 52;
            }
            return value == '+' ? 62 : value == '/' ? 63 : -1;
        };
        std::vector<std::uint8_t> bytes;
        std::uint32_t accumulator = 0;
        int bits = 0;
        for (const char character : encoded)
        {
            if (character == '=')
            {
                break;
            }
            const int value = decode(character);
            if (value < 0)
            {
                continue;
            }
            accumulator = (accumulator << 6U) | static_cast<std::uint32_t>(value);
            bits += 6;
            if (bits >= 8)
            {
                bits -= 8;
                bytes.push_back(static_cast<std::uint8_t>((accumulator >> bits) & 0xFFU));
            }
        }
        std::ofstream output(path, std::ios::binary);
        ASSERT_TRUE(output);
        output.write(reinterpret_cast<const char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
        ASSERT_TRUE(output.good());
    }

    struct TemporaryDirectory
    {
        std::filesystem::path path;

        TemporaryDirectory()
        {
            const auto uniqueValue = std::chrono::steady_clock::now().time_since_epoch().count();
            path = std::filesystem::temp_directory_path() / std::filesystem::path(u8"Fox Media Term Кириллица 图片")
                   / std::to_string(uniqueValue);
            std::filesystem::create_directories(path);
        }

        ~TemporaryDirectory()
        {
            std::error_code error;
            std::filesystem::remove_all(path, error);
        }
    };

    TEST(MediaBridge, PreparesAnOwnedImagePayloadFromAUnicodePath)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / std::filesystem::path(u8"пример 画像.ppm");
        constexpr std::array<unsigned char, 14> k_Image {
            'P', '6', '\n', '1', ' ', '1', '\n', '2', '5', '5', '\n', 255, 64, 32,
        };
        {
            std::ofstream output(imagePath, std::ios::binary);
            ASSERT_TRUE(output);
            output.write(reinterpret_cast<const char*>(k_Image.data()), k_Image.size());
        }
        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermKind kind = 0;
        FoxMediaTermBuffer buffer { };
        FoxMediaTermError error { };

        const FoxMediaTermStatus status = FoxMediaTermPreparePreview(
            pathUtf8.data(),
            pathUtf8.size(),
            FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
            1,
            1,
            { 0, 0, 4, 2 },
            &kind,
            &buffer,
            &error
        );

        ASSERT_EQ(status, FOX_MEDIA_TERM_STATUS_OK) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(kind, FOX_MEDIA_TERM_KIND_IMAGE);
        ASSERT_NE(buffer.data, nullptr);
        EXPECT_GT(buffer.size, 0U);
        EXPECT_NE(buffer.flags & FOX_MEDIA_TERM_BUFFER_ANSI_UTF8, 0U);
        FoxMediaTermReleaseBuffer(&buffer);
        FoxMediaTermReleaseError(&error);
        EXPECT_EQ(buffer.context, nullptr);
    }

    TEST(MediaBridge, ProbesImageDimensionsFromAUnicodePath)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / std::filesystem::path(u8"размер 图片.ppm");
        constexpr std::array<unsigned char, 17> k_Image {
            'P', '6', '\n', '2', ' ', '1', '\n', '2', '5', '5', '\n', 255, 0, 0, 0, 255, 0,
        };
        std::ofstream output(imagePath, std::ios::binary);
        ASSERT_TRUE(output);
        output.write(reinterpret_cast<const char*>(k_Image.data()), k_Image.size());
        output.close();

        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };
        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_IMAGE);
        EXPECT_EQ(info.pixelWidth, 2);
        EXPECT_EQ(info.pixelHeight, 1);
        EXPECT_EQ(info.flags, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ProbesAndPreparesWebpAsAnImage)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / std::filesystem::path(u8"пример 画像.WEBP");
        // Lossless 2x2 red WebP with 50% alpha.
        WriteBase64(imagePath, "UklGRhwAAABXRUJQVlA4TA8AAAAvAUAAEAcQ/Y/+BSKi/wEA");
        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_IMAGE);
        EXPECT_EQ(info.pixelWidth, 2);
        EXPECT_EQ(info.pixelHeight, 2);
        EXPECT_EQ(info.flags, 0U);

        auto decoder = FoxMediaTerm::VideoDecoder::Open(imagePath);
        ASSERT_TRUE(decoder.has_value()) << (decoder.has_value() ? "" : decoder.error());
        auto frame = decoder->ReadFrame(2, 2);
        ASSERT_TRUE(frame.has_value()) << (frame.has_value() ? "" : frame.error());
        ASSERT_TRUE(frame->has_value());
        EXPECT_EQ(frame->value().image.pixelFormat, FoxMediaTerm::EImagePixelFormat::Rgba32);
        EXPECT_EQ(frame->value().image.pixels.size(), 16U);
        EXPECT_EQ(frame->value().image.pixels[0], 255);
        EXPECT_EQ(frame->value().image.pixels[3], 127);

        FoxMediaTermKind kind = 0;
        FoxMediaTermBuffer buffer { };
        ASSERT_EQ(
            FoxMediaTermPreparePreview(
                pathUtf8.data(),
                pathUtf8.size(),
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 4, 2 },
                &kind,
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_OK
        ) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(kind, FOX_MEDIA_TERM_KIND_IMAGE);
        EXPECT_GT(buffer.size, 0U);
        FoxMediaTermReleaseBuffer(&buffer);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, RejectsInvalidWebpWithoutTreatingItAsVideo)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / "invalid.webp";
        std::ofstream(imagePath, std::ios::binary) << "not a WebP image";
        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        EXPECT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_ERROR);
        EXPECT_GT(error.size, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, DoesNotProbeVisualStudioSolutionAsAudio)
    {
        TemporaryDirectory directory;
        const std::filesystem::path solutionPath = directory.path / "project.sln";
        std::ofstream(solutionPath, std::ios::binary)
            << "Microsoft Visual Studio Solution File, Format Version 12.00\n"
            << "# Visual Studio Version 17\n"
            << "Project(\"{00000000-0000-0000-0000-000000000000}\") = \"Example\", \"Example.vcxproj\", "
               "\"{00000000-0000-0000-0000-000000000001}\"\n";

        const std::string pathUtf8 = Utf8Path(solutionPath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };
        EXPECT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_ERROR);
        EXPECT_NE(error.data, nullptr);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, DoesNotProbeTypeScriptSourceAsTransportStream)
    {
        TemporaryDirectory directory;
        const std::filesystem::path sourcePath = directory.path / "index.ts";
        std::ofstream(sourcePath, std::ios::binary) << "export interface Item { name: string; }\n"
                                                    << "export const items: Item[] = [{ name: 'Wolf' }];\n";

        const std::string pathUtf8 = Utf8Path(sourcePath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };
        EXPECT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_ERROR);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, PreparesLossyWebpAsAnImage)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / "lossy.webp";
        WriteBase64(
            imagePath,
            "UklGRjoAAABXRUJQVlA4IC4AAACQAQCdASoQABAAAUAmJaACdLoAA5gA/vtV4/+lwf/S4P/pcH/pcH8bss4bpAAA"
        );
        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermKind kind = 0;
        FoxMediaTermBuffer buffer { };
        FoxMediaTermError error { };

        ASSERT_EQ(
            FoxMediaTermPreparePreview(
                pathUtf8.data(),
                pathUtf8.size(),
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 4, 2 },
                &kind,
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_OK
        ) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(kind, FOX_MEDIA_TERM_KIND_IMAGE);
        EXPECT_GT(buffer.size, 0U);
        FoxMediaTermReleaseBuffer(&buffer);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, HandlesAnimatedWebpAccordingToFfmpegCapability)
    {
        TemporaryDirectory directory;
        const std::filesystem::path imagePath = directory.path / "animated.webp";
        WriteBase64(
            imagePath,
            "UklGRoQAAABXRUJQVlA4WAoAAAACAAAAAQAAAQAAQU5JTQYAAAD/////"
            "AQBBTk1GKAAAAAAAAAAAAAEAAAEAAGQAAAJWUDhMDwAAAC8BQAAABxDtj/4HIqL/"
            "AQBBTk1GKAAAAAAAAAAAAAEAAAEAAGQAAABWUDhMDwAAAC8BQAAABxBR//4HIqL/AQA="
        );
        const std::string pathUtf8 = Utf8Path(imagePath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        const FoxMediaTermStatus status = FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error);
        if (status == FOX_MEDIA_TERM_STATUS_OK)
        {
            EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_IMAGE);
            EXPECT_EQ(info.pixelWidth, 2);
            EXPECT_EQ(info.pixelHeight, 2);
            FoxMediaTermKind kind = 0;
            FoxMediaTermBuffer buffer { };
            ASSERT_EQ(
                FoxMediaTermPreparePreview(
                    pathUtf8.data(),
                    pathUtf8.size(),
                    FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                    1,
                    1,
                    { 0, 0, 2, 1 },
                    &kind,
                    &buffer,
                    &error
                ),
                FOX_MEDIA_TERM_STATUS_OK
            ) << std::string(error.data == nullptr ? "" : error.data, error.size);
            EXPECT_EQ(kind, FOX_MEDIA_TERM_KIND_IMAGE);
            EXPECT_GT(buffer.size, 0U);
            FoxMediaTermReleaseBuffer(&buffer);
        }
        else
        {
            EXPECT_EQ(status, FOX_MEDIA_TERM_STATUS_ERROR);
            ASSERT_NE(error.data, nullptr);
            EXPECT_NE(std::string_view(error.data, error.size).find("animated WebP"), std::string_view::npos);
        }
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, PreparesAnOwnedRgbaRasterAndValidatesItsLayout)
    {
        constexpr std::array<std::uint8_t, 20> k_Pixels {
            255, 0, 0, 255, 0, 255, 0, 255, 11, 12, 0, 0, 255, 255, 255, 255, 255, 255, 13, 14,
        };
        FoxMediaTermBuffer buffer { };
        FoxMediaTermError error { };
        EXPECT_EQ(
            FoxMediaTermPrepareRaster(
                k_Pixels.data(),
                k_Pixels.size(),
                2,
                2,
                10,
                FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8,
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 2, 1 },
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_OK
        ) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_NE(buffer.data, nullptr);
        EXPECT_GT(buffer.size, 0U);
        FoxMediaTermReleaseBuffer(&buffer);
        FoxMediaTermReleaseError(&error);

        EXPECT_EQ(
            FoxMediaTermPrepareRaster(
                k_Pixels.data(),
                k_Pixels.size(),
                2,
                2,
                10,
                FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8,
                FOX_MEDIA_TERM_PROTOCOL_KITTY,
                8,
                16,
                { 1, 2, 2, 1 },
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_OK
        ) << std::string(error.data == nullptr ? "" : error.data, error.size);
        ASSERT_NE(buffer.data, nullptr);
        EXPECT_NE(
            std::string_view(reinterpret_cast<const char*>(buffer.data), buffer.size).find("\x1b_G"),
            std::string_view::npos
        );
        FoxMediaTermReleaseBuffer(&buffer);
        FoxMediaTermReleaseError(&error);

        EXPECT_EQ(
            FoxMediaTermPrepareRaster(
                k_Pixels.data(),
                15,
                2,
                2,
                8,
                FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8,
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 2, 1 },
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_ERROR
        );
        EXPECT_GT(error.size, 0U);
        FoxMediaTermReleaseError(&error);

        EXPECT_EQ(
            FoxMediaTermPrepareRaster(
                k_Pixels.data(),
                k_Pixels.size(),
                2,
                2,
                7,
                FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8,
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 2, 1 },
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_ERROR
        );
        FoxMediaTermReleaseError(&error);

        EXPECT_EQ(
            FoxMediaTermPrepareRaster(
                nullptr,
                k_Pixels.size(),
                2,
                2,
                8,
                FOX_MEDIA_TERM_PIXEL_FORMAT_RGBA8,
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 2, 1 },
                &buffer,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_ERROR
        );
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ReturnsAUsefulErrorForAnInvalidPath)
    {
        constexpr std::string_view k_MissingPath = "missing-media-file.invalid";
        FoxMediaTermKind kind = 0;
        FoxMediaTermBuffer buffer { };
        FoxMediaTermError error { };

        const FoxMediaTermStatus status = FoxMediaTermPreparePreview(
            k_MissingPath.data(),
            k_MissingPath.size(),
            FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
            1,
            1,
            { 0, 0, 4, 2 },
            &kind,
            &buffer,
            &error
        );

        EXPECT_EQ(status, FOX_MEDIA_TERM_STATUS_ERROR);
        ASSERT_NE(error.data, nullptr);
        EXPECT_GT(error.size, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ProvidesAVideoThumbnailAndTimedFrames)
    {
        TemporaryDirectory directory;
        const std::filesystem::path videoPath = directory.path / "preview.y4m";
        constexpr std::array<std::uint8_t, 3> k_Luminance { 16, 96, 192 };
        WriteY4m(videoPath, k_Luminance);
        const std::string pathUtf8 = Utf8Path(videoPath);
        FoxMediaTermKind kind = 0;
        FoxMediaTermBuffer thumbnail { };
        FoxMediaTermError error { };

        ASSERT_EQ(
            FoxMediaTermPreparePreview(
                pathUtf8.data(),
                pathUtf8.size(),
                FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
                1,
                1,
                { 0, 0, 4, 2 },
                &kind,
                &thumbnail,
                &error
            ),
            FOX_MEDIA_TERM_STATUS_OK
        ) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(kind, FOX_MEDIA_TERM_KIND_VIDEO);
        EXPECT_GT(thumbnail.size, 0U);
        FoxMediaTermReleaseBuffer(&thumbnail);

        FoxMediaTermVideo* video = FoxMediaTermStartVideo(
            pathUtf8.data(),
            pathUtf8.size(),
            FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR,
            1,
            1,
            { 0, 0, 4, 2 },
            4,
            2,
            0,
            &error
        );
        ASSERT_NE(video, nullptr) << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(FoxMediaTermIsVideoMuted(video), 0U);
        FoxMediaTermSetVideoMuted(video, 1);
        EXPECT_EQ(FoxMediaTermIsVideoMuted(video), 1U);

        bool receivedFrame = false;
        bool finished = false;
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds { 2 };
        while (!finished && std::chrono::steady_clock::now() < deadline)
        {
            FoxMediaTermBuffer frame { };
            const FoxMediaTermStatus status = FoxMediaTermPollVideo(video, 4, 2, &frame, &error);
            ASSERT_NE(status, FOX_MEDIA_TERM_STATUS_ERROR)
                << std::string(error.data == nullptr ? "" : error.data, error.size);
            if (status == FOX_MEDIA_TERM_STATUS_OK)
            {
                receivedFrame = receivedFrame || frame.size > 0;
                FoxMediaTermReleaseBuffer(&frame);
            }
            finished = status == FOX_MEDIA_TERM_STATUS_FINISHED;
            if (!finished)
            {
                std::this_thread::sleep_for(std::chrono::milliseconds { 2 });
            }
        }
        FoxMediaTermDestroyVideo(video);
        FoxMediaTermReleaseError(&error);
        EXPECT_TRUE(receivedFrame);
        EXPECT_TRUE(finished);
    }

    TEST(MediaBridge, ProbesVideoDimensionsDurationAndMissingAudio)
    {
        TemporaryDirectory directory;
        const std::filesystem::path videoPath = directory.path / "probe.y4m";
        constexpr std::array<std::uint8_t, 3> k_Luminance { 16, 96, 192 };
        WriteY4m(videoPath, k_Luminance);
        const std::string pathUtf8 = Utf8Path(videoPath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_VIDEO);
        EXPECT_EQ(info.pixelWidth, 4);
        EXPECT_EQ(info.pixelHeight, 4);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_DURATION_KNOWN, 0U);
        EXPECT_GT(info.durationMicroseconds, 0);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_AUDIO_KNOWN, 0U);
        EXPECT_EQ(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ProbesAudioOnlyWithoutPixelDimensions)
    {
        TemporaryDirectory directory;
        const std::filesystem::path audioPath = directory.path / std::filesystem::path(u8"звук 音频.wav");
        WriteWave(audioPath);
        const std::string pathUtf8 = Utf8Path(audioPath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_AUDIO);
        EXPECT_EQ(info.pixelWidth, 0);
        EXPECT_EQ(info.pixelHeight, 0);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_DURATION_KNOWN, 0U);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_AUDIO_KNOWN, 0U);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, AttachedCoverArtDoesNotTurnAudioIntoVideo)
    {
        TemporaryDirectory directory;
        const std::filesystem::path audioPath = directory.path / "covered.mp3";
        constexpr std::string_view k_AudioWithCover =
            "SUQzBAAAAAABKVRTU0UAAAAPAAADTGF2ZjYyLjEyLjEwMQBBUElDAAAAfAAAA2ltYWdlL3BuZwAAAIlQTkcNChoKAAAADUlIRFIAAAAQ"
            "AAAAEAgCAAAAkJFoNgAAAAlwSFlzAAAAAQAAAAEATyXE1gAAACFJREFUeJxj/"
            "MtAGmAhUT3DqAZiAAtRqpDAqAZiAMmhBABoAAE7rKhBggAA"
            "AABJRU5ErkJgggAAAAAAAAAAAAD/+0DAAAAAAAAAAAAAAAAAAAAAAABJbmZvAAAADwAAAAMAAAHvAJOTk5OTk5OTk5OTk5OTk5OTk5OT"
            "k5OTk5OTk5OTk5OTk8rKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKyv///////////////////////////////////////////"
            "wAAAABMYXZjNjIuMjgAAAAAAAAAAAAAAAAkAqMAAAAAAAAB7yi2II8AAAAAAP/7EMQAAAR0E1VUkIAwpgmvNxogAgABrTlAAAFZOj1QUA"
            "gGCQHwfB8HygIAgGEQfB/UCDsTh/iDcAST9sBgOBwOAAAAAAAoiSqZFGQI6QJIFqP3hQHwExvwIpQvqBoS/CQNKgoAGDAA//sSxAKDxVg"
            "dIB3gACikg6SCvAAFzAkAvEAEhgDgeGfu9qZjA5ZhxBEmDAB+YEIGBgUgTGBeA8WatJX5SPMBMBEwAANjA2CEM25Q0y6xdzC/B9MFkB0"
            "wCwLTAlAfMCMBtE+fSQOSSAAK//sQxAKABLRDUrmSgBCXBqbrmDAEYRHSoUwW6ZrRXPoiqvkSzLu/fzeWT+BhX8eLF8DHfhVQDF2FwAAA"
            "JhKEYnPMkkGoHV5JEkKVLTyUSRQUBWMYU7xLdKgtxKpMQU1FMy4xMDCqqqo=";
        WriteBase64(audioPath, k_AudioWithCover);
        const std::string pathUtf8 = Utf8Path(audioPath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_AUDIO);
        EXPECT_EQ(info.pixelWidth, 0);
        EXPECT_EQ(info.pixelHeight, 0);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ProbesVideoWithAudio)
    {
        TemporaryDirectory directory;
        const std::filesystem::path videoPath = directory.path / std::filesystem::path(u8"звук 视频.nut");
        constexpr std::string_view k_VideoWithAudio =
            "bnV0L211bHRpbWVkaWEgY29udGFpbmVyAE5NelYfXwStYwMCgf9/"
            "AgGDkAABvkDAAAYAAAAAAAGgAAIBASgBACkAIQGffyACn397KAMAAQEpAAEEAAQBAgEEAQQBAiECAXjAAAYAAAAAAAEGAwAAAQQAAAG2Av"
            "/6Av/7Av/8Av/"
            "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"
            "AiQIUwwND/8PDQ9UDCUIAAOA/"
            "Uz4AfQh8QPwyfAEGBAQEBAQEBAQEBAQEBAQEBCAgICAgICAgAQYEBAQEBAQEBAQEBAQEBAQEICAgICAgICATljdZy8j5k4UwAACFgMFAQk"
            "AAAAAAAAAHZcaTi4=";
        WriteBase64(videoPath, k_VideoWithAudio);
        const std::string pathUtf8 = Utf8Path(videoPath);
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        ASSERT_EQ(FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error), FOX_MEDIA_TERM_STATUS_OK)
            << std::string(error.data == nullptr ? "" : error.data, error.size);
        EXPECT_EQ(info.kind, FOX_MEDIA_TERM_KIND_VIDEO);
        EXPECT_EQ(info.pixelWidth, 4);
        EXPECT_EQ(info.pixelHeight, 4);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_AUDIO_KNOWN, 0U);
        EXPECT_NE(info.flags & FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO, 0U);
        FoxMediaTermReleaseError(&error);
    }

    TEST(MediaBridge, ProbeRejectsUnsupportedFilesAndInvalidArgumentsWithoutThrowing)
    {
        TemporaryDirectory directory;
        FoxMediaTermMediaInfo info { };
        FoxMediaTermError error { };

        for (const std::string_view fileName : { "source.cpp", "settings.conf", "notes.md", "health.txt" })
        {
            const std::filesystem::path path = directory.path / fileName;
            std::ofstream(path) << "[00:01.00] text that content probing could mistake for media";
            const std::string pathUtf8 = Utf8Path(path);
            EXPECT_EQ(
                FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), &info, &error),
                FOX_MEDIA_TERM_STATUS_ERROR
            );
            ASSERT_NE(error.data, nullptr);
            EXPECT_EQ(std::string(error.data, error.size), "No media preview is available for this file type");
            FoxMediaTermReleaseError(&error);
        }
        EXPECT_EQ(error.context, nullptr);

        const std::filesystem::path path = directory.path / "plain.txt";
        const std::string pathUtf8 = Utf8Path(path);
        EXPECT_EQ(
            FoxMediaTermProbeMedia(pathUtf8.data(), pathUtf8.size(), nullptr, &error),
            FOX_MEDIA_TERM_STATUS_ERROR
        );
        EXPECT_NE(error.data, nullptr);
        FoxMediaTermReleaseError(&error);
        FoxMediaTermReleaseError(&error);
        EXPECT_EQ(error.context, nullptr);

        const std::filesystem::path corruptPath = directory.path / "corrupt.png";
        std::ofstream(corruptPath, std::ios::binary) << "\x89PNG\r\n\x1A\ntruncated";
        const std::string corruptPathUtf8 = Utf8Path(corruptPath);
        EXPECT_EQ(
            FoxMediaTermProbeMedia(corruptPathUtf8.data(), corruptPathUtf8.size(), &info, &error),
            FOX_MEDIA_TERM_STATUS_ERROR
        );
        EXPECT_NE(error.data, nullptr);
        FoxMediaTermReleaseError(&error);

        EXPECT_EQ(FoxMediaTermProbeMedia(nullptr, 1, &info, &error), FOX_MEDIA_TERM_STATUS_ERROR);
        EXPECT_NE(error.data, nullptr);
        FoxMediaTermReleaseError(&error);
        EXPECT_EQ(error.context, nullptr);
    }
} // namespace
