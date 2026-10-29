#include <algorithm>
#include <cstdint>
#include <exception>
#include <filesystem>
#include <memory>
#include <stdexcept>
#include <utility>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "AbiBoundary.h"
#include "AudioPlayback.h"
#include "MediaProbe.h"

struct FoxMediaTermAudio
{
    std::unique_ptr<FoxMediaTerm::AudioPlayback> playback;
};

extern "C"
{
    FoxMediaTermAudio* FoxMediaTermStartAudio(
        const char* pathUtf8,
        size_t pathSize,
        uint32_t muted,
        FoxMediaTermError* error
    )
    {
        Reset(error);

        try
        {
            const std::filesystem::path path = PathFromUtf8(pathUtf8, pathSize);
            auto media = FoxMediaTerm::MediaProbe::Probe(path);
            if (!media)
            {
                Store(error, media.error());
                return nullptr;
            }

            if (media->kind != FoxMediaTerm::EMediaStreamKind::Audio)
            {
                Store(error, "The selected file is not an audio-only media file");
                return nullptr;
            }

            auto playback = FoxMediaTerm::AudioPlayback::Open(path);
            if (!playback)
            {
                Store(error, playback.error());
                return nullptr;
            }

            if (!*playback)
            {
                Store(error, "The selected file contains no decodable audio stream");
                return nullptr;
            }

            (*playback)->SetMuted(muted != 0);
            return new FoxMediaTermAudio { std::move(*playback) };
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

    void FoxMediaTermDestroyAudio(FoxMediaTermAudio* audio)
    {
        delete audio;
    }

    FoxMediaTermStatus FoxMediaTermPollAudio(
        FoxMediaTermAudio* audio,
        FoxMediaTermAudioUpdate* update,
        FoxMediaTermError* error
    )
    {
        return Guard(error, [=] {
            if (audio == nullptr || update == nullptr)
            {
                throw std::invalid_argument("The audio playback handle or update output is null");
            }

            *update = { };
            if (const auto playbackError = audio->playback->Error())
            {
                Store(error, *playbackError);
                return FOX_MEDIA_TERM_STATUS_ERROR;
            }

            const auto levels = audio->playback->VisualizationLevels();
            update->positionMicroseconds = audio->playback->CurrentMediaTime().count();
            update->muted = audio->playback->IsMuted() ? 1U : 0U;
            update->levelCount = static_cast<std::uint32_t>(levels.size());
            std::ranges::copy(levels, update->levels);

            return audio->playback->IsFinished() ? FOX_MEDIA_TERM_STATUS_FINISHED : FOX_MEDIA_TERM_STATUS_OK;
        });
    }

    void FoxMediaTermSetAudioMuted(FoxMediaTermAudio* audio, uint32_t muted)
    {
        if (audio != nullptr)
        {
            audio->playback->SetMuted(muted != 0);
        }
    }

    uint32_t FoxMediaTermIsAudioMuted(const FoxMediaTermAudio* audio)
    {
        return audio != nullptr && audio->playback->IsMuted() ? 1U : 0U;
    }
}
