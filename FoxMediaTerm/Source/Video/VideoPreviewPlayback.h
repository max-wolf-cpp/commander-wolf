#pragma once

#include <expected>
#include <filesystem>
#include <memory>
#include <string>

#include "ImageRegionRenderer.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"
#include "TerminalSize.h"

namespace FoxMediaTerm
{
    struct VideoPreviewUpdate
    {
        std::string payload;
        bool usesAnsiUtf8 = false;
        bool finished = false;
    };

    class VideoPreviewPlayback
    {
      public:
        ~VideoPreviewPlayback();

        VideoPreviewPlayback(const VideoPreviewPlayback&) = delete;
        VideoPreviewPlayback& operator=(const VideoPreviewPlayback&) = delete;
        VideoPreviewPlayback(VideoPreviewPlayback&&) = delete;
        VideoPreviewPlayback& operator=(VideoPreviewPlayback&&) = delete;

        [[nodiscard]] static std::expected<std::unique_ptr<VideoPreviewPlayback>, std::string> Start(
            const std::filesystem::path& path,
            ETerminalProtocol protocol,
            TerminalCellPixelSize cellPixelSize,
            ImageCellRegion region,
            TerminalCellSize terminalSize,
            bool muted
        );

        [[nodiscard]] std::expected<VideoPreviewUpdate, std::string> Poll(TerminalCellSize terminalSize);
        void SetMuted(bool muted) noexcept;
        [[nodiscard]] bool IsMuted() const noexcept;
        [[nodiscard]] bool HasAudio() const noexcept;

      private:
        struct Implementation;

        explicit VideoPreviewPlayback(std::unique_ptr<Implementation> implementation);

        std::unique_ptr<Implementation> m_Implementation;
    };
} // namespace FoxMediaTerm
