#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include <gtest/gtest.h>

#include "AsciiGrayscaleRenderer.h"
#include "HalfBlock256Renderer.h"
#include "HalfBlockTrueColorRenderer.h"
#include "ITerm2Renderer.h"
#include "ImageLoader.h"
#include "ImagePrepareMetrics.h"
#include "ImageRegionRenderer.h"
#include "ImageResizer.h"
#include "ImageViewer.h"
#include "KittyRenderer.h"
#include "PngEncoder.h"
#include "SixelRenderer.h"

namespace
{
    std::expected<std::vector<std::uint8_t>, std::string> EncodeKnownPngPayload(
        const FoxMediaTerm::RenderableImage&,
        FoxMediaTerm::ImagePrepareMetrics*
    )
    {
        return std::vector<std::uint8_t> { 1, 2, 3 };
    }

    TEST(ImageLoader, LoadsAFileAsRgba)
    {
        const auto uniquePart = std::chrono::steady_clock::now().time_since_epoch().count();
        const std::filesystem::path imagePath =
            std::filesystem::temp_directory_path()
            / ("FoxMediaTermImageLoaderTests-" + std::to_string(uniquePart) + ".ppm");
        constexpr std::array<unsigned char, 14> k_Image {
            'P', '6', '\n', '1', ' ', '1', '\n', '2', '5', '5', '\n', 255, 0, 0,
        };

        {
            std::ofstream output(imagePath, std::ios::binary);
            ASSERT_TRUE(output);
            output.write(reinterpret_cast<const char*>(k_Image.data()), static_cast<std::streamsize>(k_Image.size()));
        }

        auto image = FoxMediaTerm::ImageLoader::Load(imagePath);
        std::error_code removeError;
        std::filesystem::remove(imagePath, removeError);

        ASSERT_TRUE(image.has_value()) << (image.has_value() ? "" : image.error());
        EXPECT_EQ(image->width, 1);
        EXPECT_EQ(image->height, 1);
        EXPECT_EQ(image->pixels, (std::vector<std::uint8_t> { 255, 0, 0, 255 }));
    }

    TEST(ImageLoader, RequiresAStbImageExtensionBeforeProbingContent)
    {
        const auto uniquePart = std::chrono::steady_clock::now().time_since_epoch().count();
        const std::filesystem::path directory = std::filesystem::temp_directory_path();
        const std::filesystem::path imagePath =
            directory / ("FoxMediaTermImageProbe-" + std::to_string(uniquePart) + ".PPM");
        const std::filesystem::path disguisedImagePath =
            directory / ("FoxMediaTermImageProbe-" + std::to_string(uniquePart) + ".data");
        const std::filesystem::path textPath =
            directory / ("FoxMediaTermImageProbe-" + std::to_string(uniquePart) + ".txt");
        constexpr std::array<unsigned char, 14> k_Image {
            'P', '6', '\n', '1', ' ', '1', '\n', '2', '5', '5', '\n', 255, 0, 0,
        };

        {
            std::ofstream imageOutput(imagePath, std::ios::binary);
            imageOutput.write(
                reinterpret_cast<const char*>(k_Image.data()),
                static_cast<std::streamsize>(k_Image.size())
            );
            std::ofstream disguisedOutput(disguisedImagePath, std::ios::binary);
            disguisedOutput.write(
                reinterpret_cast<const char*>(k_Image.data()),
                static_cast<std::streamsize>(k_Image.size())
            );
            std::ofstream(textPath) << "not an image";
        }

        EXPECT_TRUE(FoxMediaTerm::ImageLoader::HasSupportedExtension(imagePath));
        EXPECT_FALSE(FoxMediaTerm::ImageLoader::HasSupportedExtension(disguisedImagePath));
        EXPECT_TRUE(FoxMediaTerm::ImageLoader::IsSupported(imagePath));
        EXPECT_FALSE(FoxMediaTerm::ImageLoader::IsSupported(disguisedImagePath));
        EXPECT_FALSE(FoxMediaTerm::ImageLoader::IsSupported(textPath));

        std::error_code removeError;
        std::filesystem::remove(imagePath, removeError);
        std::filesystem::remove(disguisedImagePath, removeError);
        std::filesystem::remove(textPath, removeError);
    }

    TEST(ImageLoader, LoadsAPathWithSpacesCyrillicAndCjkCharacters)
    {
        const auto uniquePart = std::chrono::steady_clock::now().time_since_epoch().count();
        const std::filesystem::path parentDirectory =
            std::filesystem::temp_directory_path() / std::filesystem::path(u8"Fox Media Term Кириллица 图片");
        const std::filesystem::path testDirectory = parentDirectory / std::to_string(uniquePart);
        const std::filesystem::path imagePath = testDirectory / std::filesystem::path(u8"пример 画像.ppm");
        constexpr std::array<unsigned char, 14> k_Image {
            'P', '6', '\n', '1', ' ', '1', '\n', '2', '5', '5', '\n', 0, 255, 0,
        };

        ASSERT_TRUE(std::filesystem::create_directories(testDirectory));
        {
            std::ofstream output(imagePath, std::ios::binary);
            ASSERT_TRUE(output);
            output.write(reinterpret_cast<const char*>(k_Image.data()), static_cast<std::streamsize>(k_Image.size()));
        }

        auto image = FoxMediaTerm::ImageLoader::Load(imagePath);
        std::error_code removeError;
        std::filesystem::remove_all(testDirectory, removeError);
        std::filesystem::remove(parentDirectory, removeError);

        ASSERT_TRUE(image.has_value()) << (image.has_value() ? "" : image.error());
        EXPECT_EQ(image->pixels, (std::vector<std::uint8_t> { 0, 255, 0, 255 }));
    }

    TEST(ImageResizer, CreatesRenderableRgbaPixels)
    {
        const FoxMediaTerm::RgbaImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto resized = FoxMediaTerm::ImageResizer::Resize(image, 2, 2);

        ASSERT_TRUE(resized.has_value()) << (resized.has_value() ? "" : resized.error());
        EXPECT_EQ(resized->width, 2);
        EXPECT_EQ(resized->height, 2);
        EXPECT_EQ(resized->pixels.size(), 16);
        EXPECT_EQ(resized->pixels[0], 255);
        EXPECT_EQ(resized->pixels[3], 255);
        EXPECT_EQ(resized->pixels[12], 255);
        EXPECT_EQ(resized->pixels[15], 255);
    }

    TEST(ImageResizer, PreservesRgb24WithoutAddingAlpha)
    {
        const FoxMediaTerm::RasterImage image {
            .width = 1,
            .height = 1,
            .pixels = { 10, 20, 30 },
            .pixelFormat = FoxMediaTerm::EImagePixelFormat::Rgb24,
        };

        auto resized = FoxMediaTerm::ImageResizer::Resize(image, 2, 2);

        ASSERT_TRUE(resized.has_value()) << (resized.has_value() ? "" : resized.error());
        EXPECT_EQ(resized->pixelFormat, FoxMediaTerm::EImagePixelFormat::Rgb24);
        EXPECT_EQ(resized->pixels.size(), 12);
        EXPECT_EQ(resized->pixels[0], 10);
        EXPECT_EQ(resized->pixels[1], 20);
        EXPECT_EQ(resized->pixels[2], 30);
    }

    TEST(ImageResizer, PreparesSourceResolutionForRendering)
    {
        FoxMediaTerm::RgbaImage image {
            1,
            1,
            { 10, 20, 30, 255 },
        };

        auto renderable = FoxMediaTerm::ImageResizer::Prepare(std::move(image));

        ASSERT_TRUE(renderable.has_value()) << (renderable.has_value() ? "" : renderable.error());
        EXPECT_EQ(renderable->width, 1);
        EXPECT_EQ(renderable->height, 1);
        EXPECT_EQ(renderable->pixels, (std::vector<std::uint8_t> { 10, 20, 30, 255 }));
    }

    TEST(ImageResizer, ShrinksAnImageToFitWithoutChangingItsAspectRatio)
    {
        FoxMediaTerm::RgbaImage image {
            4,
            2,
            std::vector<std::uint8_t>(4 * 2 * 4, 255),
        };

        auto renderable = FoxMediaTerm::ImageResizer::ResizeToFit(std::move(image), 2, 2);

        ASSERT_TRUE(renderable.has_value()) << (renderable.has_value() ? "" : renderable.error());
        EXPECT_EQ(renderable->width, 2);
        EXPECT_EQ(renderable->height, 1);
    }

    TEST(ImageResizer, DoesNotUpscaleAnImageThatAlreadyFits)
    {
        FoxMediaTerm::RgbaImage image {
            2,
            1,
            std::vector<std::uint8_t>(2 * 1 * 4, 255),
        };

        auto renderable = FoxMediaTerm::ImageResizer::ResizeToFit(std::move(image), 80, 46);

        ASSERT_TRUE(renderable.has_value()) << (renderable.has_value() ? "" : renderable.error());
        EXPECT_EQ(renderable->width, 2);
        EXPECT_EQ(renderable->height, 1);
    }

    TEST(ImageRegionRenderer, FitsPixelAndTextBackendsToTheSameCellRegion)
    {
        FoxMediaTerm::RgbaImage pixelImage {
            100,
            100,
            std::vector<std::uint8_t>(100 * 100 * 4, 255),
        };
        FoxMediaTerm::RgbaImage textImage = pixelImage;
        const FoxMediaTerm::ImageCellRegion region { 10, 4, 5, 2 };

        auto kitty = FoxMediaTerm::ImageRegionRenderer::Prepare(
            std::move(pixelImage),
            FoxMediaTerm::ETerminalProtocol::Kitty,
            { 10, 20, true },
            region
        );
        auto halfBlocks = FoxMediaTerm::ImageRegionRenderer::Prepare(
            std::move(textImage),
            FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor,
            { 10, 20, true },
            region
        );

        ASSERT_TRUE(kitty.has_value()) << (kitty ? "" : kitty.error());
        ASSERT_TRUE(halfBlocks.has_value()) << (halfBlocks ? "" : halfBlocks.error());
        EXPECT_EQ(kitty->pixelWidth, 40);
        EXPECT_EQ(kitty->pixelHeight, 40);
        EXPECT_EQ(halfBlocks->pixelWidth, 4);
        EXPECT_EQ(halfBlocks->pixelHeight, 4);
        EXPECT_NE(kitty->payload.find("\x1b[5;11H"), std::string::npos);
        EXPECT_NE(halfBlocks->payload.find("\x1b[5;11H"), std::string::npos);
    }

    TEST(ImageRegionRenderer, PreparesPositionedKittyPingPongFramesWithoutClearingThePanel)
    {
        FoxMediaTerm::RgbaImage image {
            2,
            2,
            std::vector<std::uint8_t>(2 * 2 * 4, 255),
        };
        auto prepared = FoxMediaTerm::ImageRegionRenderer::Prepare(
            std::move(image),
            FoxMediaTerm::ETerminalProtocol::Kitty,
            { 8, 16, true },
            { 5, 3, 10, 6 },
            &EncodeKnownPngPayload,
            nullptr,
            FoxMediaTerm::EKittyPreparationMode::PingPongVideo
        );

        ASSERT_TRUE(prepared.has_value()) << (prepared ? "" : prepared.error());
        EXPECT_FALSE(prepared->payload.empty());
        EXPECT_FALSE(prepared->alternatePayload.empty());
        EXPECT_EQ(prepared->payload.find(FoxMediaTerm::KittyRenderer::DeleteAll()), std::string::npos);
        EXPECT_EQ(prepared->alternatePayload.find(FoxMediaTerm::KittyRenderer::DeleteAll()), std::string::npos);
        EXPECT_NE(prepared->payload.find("\x1b["), std::string::npos);
        EXPECT_NE(prepared->alternatePayload.find("\x1b["), std::string::npos);
    }

    TEST(ImageViewer, PreparesValidOutputForEveryResolvedProtocol)
    {
        constexpr std::array k_Protocols {
            FoxMediaTerm::ETerminalProtocol::Kitty,        FoxMediaTerm::ETerminalProtocol::ITerm2,
            FoxMediaTerm::ETerminalProtocol::Sixel,        FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor,
            FoxMediaTerm::ETerminalProtocol::HalfBlock256, FoxMediaTerm::ETerminalProtocol::AsciiGrayscale,
        };

        for (const FoxMediaTerm::ETerminalProtocol protocol : k_Protocols)
        {
            FoxMediaTerm::RgbaImage image {
                2,
                2,
                {
                    255,
                    0,
                    0,
                    255,
                    0,
                    255,
                    0,
                    255,
                    0,
                    0,
                    255,
                    255,
                    255,
                    255,
                    255,
                    255,
                },
            };

            FoxMediaTerm::ImagePrepareMetrics metrics;
            auto view =
                FoxMediaTerm::ImageViewer::Prepare(std::move(image), protocol, { 10, 5 }, { 8, 16, true }, &metrics);

            ASSERT_TRUE(view.has_value()) << (view.has_value() ? "" : view.error());
            EXPECT_FALSE(view->payload.empty());
            EXPECT_TRUE(
                metrics.stageRecorded[static_cast<std::size_t>(FoxMediaTerm::EImagePrepareStage::InputConversion)]
            );
            EXPECT_TRUE(
                metrics.stageRecorded[static_cast<std::size_t>(FoxMediaTerm::EImagePrepareStage::BackendEncode)]
            );
            EXPECT_TRUE(
                metrics.stageRecorded[static_cast<std::size_t>(FoxMediaTerm::EImagePrepareStage::FinalPayloadAssembly)]
            );
            EXPECT_TRUE(
                metrics.stageRecorded[static_cast<std::size_t>(FoxMediaTerm::EImagePrepareStage::PrepareTotal)]
            );
            EXPECT_EQ(metrics.inputPixels, 4);
            EXPECT_EQ(metrics.inputBytes, 16);
            EXPECT_EQ(metrics.finalPayloadBytes, view->payload.size());
            EXPECT_GE(metrics.finalPayloadCapacity, metrics.finalPayloadBytes);

            const auto stageRecorded = [&metrics](FoxMediaTerm::EImagePrepareStage stage) {
                return metrics.stageRecorded[static_cast<std::size_t>(stage)];
            };
            switch (protocol)
            {
                case FoxMediaTerm::ETerminalProtocol::Kitty:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::PngEncode));
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::Base64Encode));
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::KittyFramingAssembly));
                    break;
                case FoxMediaTerm::ETerminalProtocol::ITerm2:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::PngEncode));
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::Base64Encode));
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::ITerm2FramingAssembly));
                    break;
                case FoxMediaTerm::ETerminalProtocol::Sixel:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::SixelPaletteQuantization));
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::SixelDitherEncode));
                    break;
                case FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::HalfBlockTrueColorCellsAnsiAssembly));
                    break;
                case FoxMediaTerm::ETerminalProtocol::HalfBlock256:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::HalfBlock256CellsAnsiAssembly));
                    break;
                case FoxMediaTerm::ETerminalProtocol::AsciiGrayscale:
                    EXPECT_TRUE(stageRecorded(FoxMediaTerm::EImagePrepareStage::AsciiCellsAssembly));
                    break;
                case FoxMediaTerm::ETerminalProtocol::Auto:
                    FAIL() << "Auto is not a resolved image protocol";
                    break;
            }
        }
    }

    TEST(ImageViewer, PreparesRgb24ForEveryResolvedProtocol)
    {
        constexpr std::array k_Protocols {
            FoxMediaTerm::ETerminalProtocol::Kitty,        FoxMediaTerm::ETerminalProtocol::ITerm2,
            FoxMediaTerm::ETerminalProtocol::Sixel,        FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor,
            FoxMediaTerm::ETerminalProtocol::HalfBlock256, FoxMediaTerm::ETerminalProtocol::AsciiGrayscale,
        };
        const FoxMediaTerm::RasterImage image {
            .width = 2,
            .height = 2,
            .pixels = { 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255 },
            .pixelFormat = FoxMediaTerm::EImagePixelFormat::Rgb24,
        };

        for (const FoxMediaTerm::ETerminalProtocol protocol : k_Protocols)
        {
            FoxMediaTerm::ImagePrepareMetrics metrics;
            auto view = FoxMediaTerm::ImageViewer::Prepare(image, protocol, { 10, 5 }, { 8, 16, true }, &metrics);

            ASSERT_TRUE(view.has_value()) << (view.has_value() ? "" : view.error());
            EXPECT_FALSE(view->payload.empty());
            EXPECT_EQ(metrics.inputPixels, 4);
            EXPECT_EQ(metrics.inputBytes, 12);
        }
    }

    TEST(ITerm2Renderer, EncodesInlinePngEscapeStream)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto stream = FoxMediaTerm::ITerm2Renderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_TRUE(stream->starts_with("\x1b]1337;File=inline=1;size="));
        EXPECT_NE(stream->find(";width=1px;height=1px;preserveAspectRatio=1:iVBORw0KGgo"), std::string::npos);
        EXPECT_TRUE(stream->ends_with("\x1b\\\n"));
    }

    TEST(ImageEncoders, ReportTheEffectiveLibraryConfiguration)
    {
        const FoxMediaTerm::PngEncoderConfiguration png = FoxMediaTerm::PngEncoder::Configuration();
        EXPECT_EQ(png.implementation, "stb_image_write");
        EXPECT_EQ(png.compressionLevel, 8);
        EXPECT_EQ(png.forcedFilter, -1);
        EXPECT_FALSE(png.reusesContext);
        EXPECT_FALSE(png.exposesFilterAndDeflateTimings);

        const FoxMediaTerm::SixelEncoderConfiguration sixel = FoxMediaTerm::SixelRenderer::Configuration();
        EXPECT_NE(sixel.implementation.find("libsixel"), std::string::npos);
        EXPECT_EQ(sixel.paletteColors, 256);
        EXPECT_EQ(sixel.largestDimensionAlgorithm, "LARGE_NORM");
        EXPECT_EQ(sixel.representativeColorAlgorithm, "REP_AUTO");
        EXPECT_EQ(sixel.qualityMode, "QUALITY_AUTO");
        EXPECT_NE(sixel.diffusionAlgorithm.find("Floyd-Steinberg"), std::string::npos);
        EXPECT_FALSE(sixel.reusesContext);
        EXPECT_FALSE(sixel.exposesDitherAndPackingTimings);
    }

    TEST(ImageViewer, UsesTheProvidedPngEncoderWithoutChangingTheDefaultImageEncoder)
    {
        const FoxMediaTerm::RasterImage image {
            .width = 1,
            .height = 1,
            .pixels = { 255, 0, 0 },
            .pixelFormat = FoxMediaTerm::EImagePixelFormat::Rgb24,
        };

        auto view = FoxMediaTerm::ImageViewer::Prepare(
            image,
            FoxMediaTerm::ETerminalProtocol::ITerm2,
            { 10, 5 },
            { 8, 16, true },
            nullptr,
            FoxMediaTerm::EKittyPreparationMode::SingleImage,
            EncodeKnownPngPayload
        );

        ASSERT_TRUE(view.has_value()) << (view.has_value() ? "" : view.error());
        EXPECT_NE(view->payload.find("size=3"), std::string::npos);
        EXPECT_NE(view->payload.find(":AQID"), std::string::npos);
        EXPECT_EQ(FoxMediaTerm::PngEncoder::Configuration().implementation, "stb_image_write");
    }

    TEST(ITerm2Renderer, CanKeepTheCursorFixedForRepeatedFrames)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto stream = FoxMediaTerm::ITerm2Renderer::Encode(image, { .preserveCursor = true });

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_NE(stream->find(";preserveAspectRatio=1;doNotMoveCursor=1:"), std::string::npos);
    }

    TEST(ITerm2Renderer, CanDisplayATransmissionAtIndependentPixelDimensions)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto stream = FoxMediaTerm::ITerm2Renderer::Encode(
            image,
            {
                .displayWidthPixels = 640,
                .displayHeightPixels = 360,
                .preserveAspectRatio = false,
            }
        );

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_NE(stream->find(";width=640px;height=360px;preserveAspectRatio=0:"), std::string::npos);
    }

    TEST(ITerm2Renderer, CanAddressTheDisplayAreaInTerminalCells)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto stream = FoxMediaTerm::ITerm2Renderer::Encode(
            image,
            {
                .displayColumns = 80,
                .displayRows = 24,
                .preserveAspectRatio = false,
            }
        );

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_NE(stream->find(";width=80;height=24;preserveAspectRatio=0:"), std::string::npos);
    }

    TEST(SixelRenderer, EncodesRgbaDirectlyWithLibsixel)
    {
        FoxMediaTerm::RenderableImage image {
            2,
            1,
            { 255, 0, 0, 255, 0, 0, 255, 255 },
        };

        auto stream = FoxMediaTerm::SixelRenderer::Encode(std::move(image));

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_TRUE(stream->starts_with("\x1bPq"));
        EXPECT_TRUE(stream->ends_with("\x1b\\\n"));
    }

    TEST(KittyRenderer, EncodesInlinePngGraphicsCommand)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto stream = FoxMediaTerm::KittyRenderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_TRUE(stream->starts_with("\x1b_Ga=T,f=100,m=0;iVBORw0KGgo"));
        EXPECT_TRUE(stream->ends_with("\x1b\\"));
        EXPECT_EQ(FoxMediaTerm::KittyRenderer::DeleteAll(), "\x1b_Ga=d,d=A\x1b\\");
    }

    TEST(KittyRenderer, SplitsLargePayloadsIntoProtocolChunks)
    {
        FoxMediaTerm::RenderableImage image {
            64,
            64,
            std::vector<std::uint8_t>(64 * 64 * 4),
        };
        std::uint32_t randomState = 0x12345678;
        for (std::size_t index = 0; index < image.pixels.size(); index += 4)
        {
            randomState ^= randomState << 13;
            randomState ^= randomState >> 17;
            randomState ^= randomState << 5;
            image.pixels[index] = static_cast<std::uint8_t>(randomState);
            image.pixels[index + 1] = static_cast<std::uint8_t>(randomState >> 8);
            image.pixels[index + 2] = static_cast<std::uint8_t>(randomState >> 16);
            image.pixels[index + 3] = 255;
        }

        auto stream = FoxMediaTerm::KittyRenderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        constexpr std::string_view k_CommandStart = "\x1b_G";
        constexpr std::string_view k_CommandEnd = "\x1b\\";
        std::size_t commandOffset = 0;
        std::size_t commandCount = 0;
        while (commandOffset < stream->size())
        {
            ASSERT_EQ(stream->find(k_CommandStart, commandOffset), commandOffset);
            const std::size_t separator = stream->find(';', commandOffset + k_CommandStart.size());
            ASSERT_NE(separator, std::string::npos);
            const std::size_t commandEnd = stream->find(k_CommandEnd, separator + 1);
            ASSERT_NE(commandEnd, std::string::npos);

            const std::string_view controlData(
                stream->data() + commandOffset + k_CommandStart.size(),
                separator - commandOffset - k_CommandStart.size()
            );
            const std::size_t payloadSize = commandEnd - separator - 1;
            EXPECT_LE(payloadSize, 4096);
            if (commandCount == 0)
            {
                EXPECT_EQ(controlData, "a=T,f=100,m=1");
            }
            else if (commandEnd + k_CommandEnd.size() < stream->size())
            {
                EXPECT_EQ(controlData, "m=1");
                EXPECT_EQ(payloadSize % 4, 0);
            }
            else
            {
                EXPECT_EQ(controlData, "m=0");
            }

            commandOffset = commandEnd + k_CommandEnd.size();
            ++commandCount;
        }

        EXPECT_GT(commandCount, 1);
    }

    TEST(ImageViewer, PreparesKittyPingPongInTransmitPlaceDeleteOrder)
    {
        const FoxMediaTerm::RgbaImage image {
            1,
            1,
            { 255, 0, 0, 255 },
        };

        auto view = FoxMediaTerm::ImageViewer::Prepare(
            image,
            FoxMediaTerm::ETerminalProtocol::Kitty,
            { 10, 5 },
            { 8, 16, true },
            nullptr,
            FoxMediaTerm::EKittyPreparationMode::PingPongVideo
        );

        ASSERT_TRUE(view.has_value()) << (view.has_value() ? "" : view.error());
        ASSERT_FALSE(view->alternatePayload.empty());
        EXPECT_EQ(view->payload.find(FoxMediaTerm::KittyRenderer::DeleteAll()), std::string::npos);
        EXPECT_EQ(view->alternatePayload.find(FoxMediaTerm::KittyRenderer::DeleteAll()), std::string::npos);

        const std::string firstTransmit =
            "a=t,f=100,q=2,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_FirstImageId);
        const std::string firstPlace = "a=p,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_FirstImageId)
                                       + ",p=" + std::to_string(FoxMediaTerm::KittyRenderer::k_FirstPlacementId);
        const std::string deleteSecond = "a=d,d=I,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_SecondImageId);
        const std::size_t transmitOffset = view->payload.find(firstTransmit);
        const std::size_t placeOffset = view->payload.find(firstPlace);
        const std::size_t deleteOffset = view->payload.find(deleteSecond);
        ASSERT_NE(transmitOffset, std::string::npos);
        ASSERT_NE(placeOffset, std::string::npos);
        ASSERT_NE(deleteOffset, std::string::npos);
        EXPECT_LT(transmitOffset, placeOffset);
        EXPECT_LT(view->payload.rfind("m=0;"), placeOffset);
        EXPECT_LT(placeOffset, deleteOffset);
        EXPECT_EQ(view->kittyDeleteSuffixBytes, view->payload.size() - deleteOffset + 3);

        const std::string secondTransmit =
            "a=t,f=100,q=2,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_SecondImageId);
        const std::string secondPlace = "a=p,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_SecondImageId)
                                        + ",p=" + std::to_string(FoxMediaTerm::KittyRenderer::k_SecondPlacementId);
        const std::string deleteFirst = "a=d,d=I,i=" + std::to_string(FoxMediaTerm::KittyRenderer::k_FirstImageId);
        EXPECT_LT(view->alternatePayload.find(secondTransmit), view->alternatePayload.find(secondPlace));
        EXPECT_LT(view->alternatePayload.rfind("m=0;"), view->alternatePayload.find(secondPlace));
        EXPECT_LT(view->alternatePayload.find(secondPlace), view->alternatePayload.find(deleteFirst));
    }

    TEST(HalfBlockTrueColorRenderer, CombinesTwoPixelRowsIntoOneTerminalRow)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            2,
            { 255, 0, 0, 255, 0, 0, 255, 255 },
        };

        auto stream = FoxMediaTerm::HalfBlockTrueColorRenderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_EQ(*stream, "\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m\xE2\x96\x80\x1b[0m\n");
    }

    TEST(HalfBlockTrueColorRenderer, UsesDefaultBackgroundForAnUnpairedTopRow)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 100, 50, 200, 128 },
        };

        auto stream = FoxMediaTerm::HalfBlockTrueColorRenderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_EQ(*stream, "\x1b[38;2;50;25;100m\x1b[49m\xE2\x96\x80\x1b[0m\n");
    }

    TEST(HalfBlock256Renderer, CombinesTwoPixelRowsUsingTheFixedColorCube)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            2,
            { 255, 0, 0, 255, 0, 0, 255, 255 },
        };

        auto stream = FoxMediaTerm::HalfBlock256Renderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_EQ(*stream, "\x1b[38;5;196m\x1b[48;5;21m\xE2\x96\x80\x1b[0m\n");
    }

    TEST(HalfBlock256Renderer, UsesTheGrayscaleRampAndDefaultBackground)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            1,
            { 255, 255, 255, 128 },
        };

        auto stream = FoxMediaTerm::HalfBlock256Renderer::Encode(image);

        ASSERT_TRUE(stream.has_value()) << (stream.has_value() ? "" : stream.error());
        EXPECT_EQ(*stream, "\x1b[38;5;244m\x1b[49m\xE2\x96\x80\x1b[0m\n");
    }

    TEST(AsciiGrayscaleRenderer, MapsBlackAndWhiteToTheRampEndpoints)
    {
        const FoxMediaTerm::RenderableImage image {
            2,
            1,
            { 0, 0, 0, 255, 255, 255, 255, 255 },
        };

        auto text = FoxMediaTerm::AsciiGrayscaleRenderer::Encode(image);

        ASSERT_TRUE(text.has_value()) << (text.has_value() ? "" : text.error());
        EXPECT_EQ(*text, " @\n");
    }

    TEST(AsciiGrayscaleRenderer, AveragesTwoPixelRowsAndCompositesAlpha)
    {
        const FoxMediaTerm::RenderableImage image {
            1,
            2,
            { 0, 0, 0, 255, 255, 255, 255, 128 },
        };

        auto text = FoxMediaTerm::AsciiGrayscaleRenderer::Encode(image);

        ASSERT_TRUE(text.has_value()) << (text.has_value() ? "" : text.error());
        EXPECT_EQ(*text, ":\n");
    }
} // namespace
