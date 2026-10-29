#include "MediaPreview.h"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <expected>
#include <filesystem>
#include <stdexcept>
#include <string>
#include <utility>

#include "AbiBoundary.h"
#include "ImageLoader.h"
#include "ImageRegionRenderer.h"
#include "MediaProbe.h"
#include "VideoDecoder.h"
#include "VideoThumbnail.h"

namespace
{
    using FoxMediaTerm::ImageRegionRenderer;
    using FoxMediaTerm::RasterImage;

    [[nodiscard]] bool IsWebpImage(const std::filesystem::path& path);
    [[nodiscard]] std::expected<RasterImage, std::string> LoadWebpImage(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    );
    [[nodiscard]] std::expected<std::pair<RasterImage, FoxMediaTermKind>, std::string> LoadVideoThumbnail(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    );
    FoxMediaTermStatus ProbeWebpImage(
        const std::filesystem::path& path,
        FoxMediaTermMediaInfo& info,
        FoxMediaTermError* error
    );
    FoxMediaTermStatus ProbeImageDimensions(
        const std::filesystem::path& path,
        FoxMediaTermMediaInfo& info,
        FoxMediaTermError* error
    );
} // namespace

namespace FoxMediaTerm::Bridge
{
    [[nodiscard]] std::expected<std::pair<RasterImage, FoxMediaTermKind>, std::string> LoadPreview(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    )
    {
        if (IsWebpImage(path))
        {
            auto image = LoadWebpImage(path, protocol, cellPixelSize, region);
            if (!image)
            {
                return std::unexpected(image.error());
            }

            return std::pair<RasterImage, FoxMediaTermKind> {
                std::move(*image),
                FOX_MEDIA_TERM_KIND_IMAGE,
            };
        }

        if (FoxMediaTerm::ImageLoader::HasSupportedExtension(path))
        {
            auto image = FoxMediaTerm::ImageLoader::Load(path);
            if (!image)
            {
                return std::unexpected(image.error());
            }

            return std::pair<RasterImage, FoxMediaTermKind> {
                std::move(*image),
                FOX_MEDIA_TERM_KIND_IMAGE,
            };
        }

        return LoadVideoThumbnail(path, protocol, cellPixelSize, region);
    }
} // namespace FoxMediaTerm::Bridge

extern "C"
{
    FoxMediaTermStatus FoxMediaTermProbeMedia(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermMediaInfo* info,
        FoxMediaTermError* error
    )
    {
        return Guard(error, [=]() -> FoxMediaTermStatus {
            if (info == nullptr)
            {
                throw std::invalid_argument("The media information output is null");
            }

            *info = { };
            const std::filesystem::path path = PathFromUtf8(pathUtf8, pathSize);
            if (IsWebpImage(path))
            {
                return ProbeWebpImage(path, *info, error);
            }

            if (FoxMediaTerm::ImageLoader::HasSupportedExtension(path))
            {
                return ProbeImageDimensions(path, *info, error);
            }

            if (!FoxMediaTerm::MediaProbe::HasSupportedExtension(path))
            {
                Store(error, "No media preview is available for this file type");
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            auto media = FoxMediaTerm::MediaProbe::Probe(path);
            if (!media)
            {
                Store(error, media.error());
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            std::uint32_t flags = FOX_MEDIA_TERM_MEDIA_INFO_AUDIO_KNOWN;
            if (media->duration)
            {
                flags |= FOX_MEDIA_TERM_MEDIA_INFO_DURATION_KNOWN;
            }
            if (media->hasAudio)
            {
                flags |= FOX_MEDIA_TERM_MEDIA_INFO_HAS_AUDIO;
            }

            *info = {
                .kind = static_cast<FoxMediaTermKind>(
                    media->kind == FoxMediaTerm::EMediaStreamKind::Video ? FOX_MEDIA_TERM_KIND_VIDEO
                                                                         : FOX_MEDIA_TERM_KIND_AUDIO
                ),
                .pixelWidth = media->width,
                .pixelHeight = media->height,
                .durationMicroseconds = media->duration ? media->duration->count() : 0,
                .flags = flags,
            };

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }
}

namespace
{
    [[nodiscard]] bool IsWebpImage(const std::filesystem::path& path)
    {
        std::string extension = path.extension().string();
        std::ranges::transform(extension, extension.begin(), [](unsigned char character) {
            return static_cast<char>(std::tolower(character));
        });

        return extension == ".webp";
    }

    [[nodiscard]] std::expected<RasterImage, std::string> LoadWebpImage(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    )
    {
        auto decoder = FoxMediaTerm::VideoDecoder::Open(path);
        if (!decoder)
        {
            return std::unexpected(decoder.error());
        }

        const FoxMediaTerm::ImagePixelSize maximumSize =
            ImageRegionRenderer::MaximumPixelSize(protocol, cellPixelSize, region);
        auto frame = decoder->ReadFrame(maximumSize.width, maximumSize.height);
        if (!frame)
        {
            return std::unexpected(frame.error());
        }

        if (!*frame)
        {
            return std::unexpected("The WebP image contains no decodable frames");
        }

        return std::move(frame->value().image);
    }

    [[nodiscard]] std::expected<std::pair<RasterImage, FoxMediaTermKind>, std::string> LoadVideoThumbnail(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    )
    {
        if (!FoxMediaTerm::MediaProbe::HasSupportedExtension(path))
        {
            return std::unexpected("No media preview is available for this file type");
        }

        auto media = FoxMediaTerm::MediaProbe::Probe(path);
        if (!media)
        {
            return std::unexpected(media.error());
        }

        if (media->kind != FoxMediaTerm::EMediaStreamKind::Video)
        {
            return std::unexpected("Audio files do not have a terminal image preview");
        }

        auto decoder = FoxMediaTerm::VideoDecoder::Open(path);
        if (!decoder)
        {
            return std::unexpected(decoder.error());
        }

        const FoxMediaTerm::ImagePixelSize maximumSize =
            ImageRegionRenderer::MaximumPixelSize(protocol, cellPixelSize, region);
        auto thumbnail = FoxMediaTerm::VideoThumbnail::Read(*decoder, maximumSize.width, maximumSize.height);
        if (!thumbnail)
        {
            return std::unexpected(thumbnail.error());
        }

        if (!*thumbnail)
        {
            return std::unexpected("The video contains no previewable frames");
        }

        return std::pair<RasterImage, FoxMediaTermKind> {
            std::move(**thumbnail),
            FOX_MEDIA_TERM_KIND_VIDEO,
        };
    }

    FoxMediaTermStatus ProbeWebpImage(
        const std::filesystem::path& path,
        FoxMediaTermMediaInfo& info,
        FoxMediaTermError* error
    )
    {
        auto media = FoxMediaTerm::VideoDecoder::Probe(path);
        if (!media)
        {
            Store(error, media.error());
            return FOX_MEDIA_TERM_STATUS_ERROR;
        }

        info = {
            .kind = FOX_MEDIA_TERM_KIND_IMAGE,
            .pixelWidth = media->width,
            .pixelHeight = media->height,
            .durationMicroseconds = 0,
            .flags = 0,
        };

        return FOX_MEDIA_TERM_STATUS_OK;
    }

    FoxMediaTermStatus ProbeImageDimensions(
        const std::filesystem::path& path,
        FoxMediaTermMediaInfo& info,
        FoxMediaTermError* error
    )
    {
        auto dimensions = FoxMediaTerm::ImageLoader::ProbeDimensions(path);
        if (!dimensions)
        {
            Store(error, dimensions.error());
            return FOX_MEDIA_TERM_STATUS_ERROR;
        }

        info = {
            .kind = FOX_MEDIA_TERM_KIND_IMAGE,
            .pixelWidth = dimensions->first,
            .pixelHeight = dimensions->second,
            .durationMicroseconds = 0,
            .flags = 0,
        };

        return FOX_MEDIA_TERM_STATUS_OK;
    }
} // namespace
