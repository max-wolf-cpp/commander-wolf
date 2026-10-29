#include <cstdint>
#include <exception>
#include <memory>
#include <stdexcept>
#include <utility>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "AbiBoundary.h"
#include "VideoPreviewPlayback.h"

struct FoxMediaTermVideo
{
    std::unique_ptr<FoxMediaTerm::VideoPreviewPlayback> playback;
};

extern "C"
{
    FoxMediaTermVideo* FoxMediaTermStartVideo(
        const char* pathUtf8,
        size_t pathSize,
        FoxMediaTermProtocol protocol,
        int32_t cellPixelWidth,
        int32_t cellPixelHeight,
        FoxMediaTermRegion region,
        int32_t terminalColumns,
        int32_t terminalRows,
        uint32_t muted,
        FoxMediaTermError* error
    )
    {
        Reset(error);

        try
        {
            auto playback = FoxMediaTerm::VideoPreviewPlayback::Start(
                PathFromUtf8(pathUtf8, pathSize),
                ProtocolFromAbi(protocol),
                CellSize(cellPixelWidth, cellPixelHeight),
                RegionFromAbi(region),
                { terminalColumns, terminalRows },
                muted != 0
            );
            if (!playback)
            {
                Store(error, playback.error());
                return nullptr;
            }

            return new FoxMediaTermVideo { std::move(*playback) };
        }
        catch (const std::exception& exception)
        {
            Store(error, exception.what());
        }
        catch (...)
        {
            Store(error, "The media library failed with an unknown exception");
        }

        return nullptr;
    }

    void FoxMediaTermDestroyVideo(FoxMediaTermVideo* video)
    {
        delete video;
    }

    FoxMediaTermStatus FoxMediaTermPollVideo(
        FoxMediaTermVideo* video,
        int32_t terminalColumns,
        int32_t terminalRows,
        FoxMediaTermBuffer* buffer,
        FoxMediaTermError* error
    )
    {
        Reset(buffer);

        return Guard(error, [=] {
            if (video == nullptr || buffer == nullptr)
            {
                throw std::invalid_argument("The video playback handle or output is null");
            }

            auto update = video->playback->Poll({ terminalColumns, terminalRows });
            if (!update)
            {
                Store(error, update.error());
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            if (!update->payload.empty())
            {
                Store(*buffer, std::move(update->payload), update->usesAnsiUtf8);
                return FOX_MEDIA_TERM_STATUS_OK;
            }

            return update->finished ? FOX_MEDIA_TERM_STATUS_FINISHED : FOX_MEDIA_TERM_STATUS_NO_FRAME;
        });
    }

    void FoxMediaTermSetVideoMuted(FoxMediaTermVideo* video, uint32_t muted)
    {
        if (video != nullptr)
        {
            video->playback->SetMuted(muted != 0);
        }
    }

    uint32_t FoxMediaTermIsVideoMuted(const FoxMediaTermVideo* video)
    {
        return video != nullptr && video->playback->IsMuted() ? 1U : 0U;
    }

    uint32_t FoxMediaTermVideoHasAudio(const FoxMediaTermVideo* video)
    {
        return video != nullptr && video->playback->HasAudio() ? 1U : 0U;
    }
}
