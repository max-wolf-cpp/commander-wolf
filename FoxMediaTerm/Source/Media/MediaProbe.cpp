#include "MediaProbe.h"

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <memory>
#include <string>
#include <string_view>

extern "C"
{
#include <libavformat/avformat.h>
#include <libavutil/error.h>
#include <libavutil/log.h>
}

namespace FoxMediaTerm
{
    namespace
    {
        constexpr int k_MicrosecondsPerSecond = 1'000'000;

        [[nodiscard]] std::string PathForFfmpeg(const std::filesystem::path& path);
        [[nodiscard]] std::string LowercaseExtension(const std::filesystem::path& path);
        [[nodiscard]] bool IsAudioOrVideoExtension(std::string_view extension);
        [[nodiscard]] std::string FfmpegError(std::string_view operation, int errorCode);
        [[nodiscard]] std::expected<ProbedMediaInfo, std::string> InspectStreams(
            const AVFormatContext& format,
            const std::filesystem::path& path
        );
    } // namespace

    bool MediaProbe::HasSupportedExtension(const std::filesystem::path& path)
    {
        const std::string extension = LowercaseExtension(path);

        // WebP is decoded by FFmpeg through the image path.
        return extension == "webp" || IsAudioOrVideoExtension(extension);
    }

    std::expected<ProbedMediaInfo, std::string> MediaProbe::Probe(const std::filesystem::path& path)
    {
        av_log_set_level(AV_LOG_QUIET);
        if (!HasSupportedExtension(path))
        {
            return std::unexpected("Unsupported media file extension: " + PathForFfmpeg(path));
        }

        AVFormatContext* rawContext = nullptr;
        const std::string pathText = PathForFfmpeg(path);
        const int openResult = avformat_open_input(&rawContext, pathText.c_str(), nullptr, nullptr);
        if (openResult < 0)
        {
            return std::unexpected(FfmpegError("Opening media '" + pathText + "'", openResult));
        }

        const auto closeContext = [](AVFormatContext* context) { avformat_close_input(&context); };
        std::unique_ptr<AVFormatContext, decltype(closeContext)> formatContext(rawContext, closeContext);

        const int streamInfoResult = avformat_find_stream_info(formatContext.get(), nullptr);
        if (streamInfoResult < 0)
        {
            return std::unexpected(FfmpegError("Reading media stream information", streamInfoResult));
        }

        return InspectStreams(*formatContext, path);
    }

    namespace
    {
        [[nodiscard]] std::string PathForFfmpeg(const std::filesystem::path& path)
        {
            const std::u8string text = path.generic_u8string();
            return std::string(reinterpret_cast<const char*>(text.data()), text.size());
        }

        [[nodiscard]] std::string LowercaseExtension(const std::filesystem::path& path)
        {
            std::string extension = path.extension().string();
            if (!extension.empty() && extension.front() == '.')
            {
                extension.erase(extension.begin());
            }

            std::ranges::transform(extension, extension.begin(), [](unsigned char character) {
                return static_cast<char>(std::tolower(character));
            });

            return extension;
        }

        [[nodiscard]] bool IsAudioOrVideoExtension(std::string_view extension)
        {
            // FFmpeg advertises extensions for raw samples, subtitles and unrelated file types as well.
            // Only route recognizable audio and video formats to its probe; the probe still validates content.
            static constexpr std::array k_Extensions {
                "3g2",   "3gp",  "aa",   "aac",  "ac3",  "ac4",  "aif",  "aifc",  "aiff", "ape",  "asf", "au",  "avi",
                "av1",   "avc",  "caf",  "dff",  "dif",  "dsf",  "dts",  "dtshd", "dv",   "eac3", "ec3", "f4v", "flac",
                "flv",   "h264", "h265", "hevc", "m1v",  "m2a",  "m2ts", "m2v",   "m4a",  "m4b",  "m4v", "mka", "mkv",
                "mjpeg", "mjpg", "mov",  "mp2",  "mp3",  "mp4",  "mpa",  "mpe",   "mpeg", "mpg",  "mts", "mxf", "nut",
                "obu",   "oga",  "ogg",  "ogv",  "opus", "qoa",  "ra",   "rm",    "rmvb", "shn",  "tak", "thd", "ts",
                "tta",   "vob",  "vqf",  "wav",  "wave", "webm", "wma",  "wmv",   "wv",   "y4m",
            };

            return std::ranges::find(k_Extensions, extension) != k_Extensions.end();
        }

        [[nodiscard]] std::string FfmpegError(std::string_view operation, int errorCode)
        {
            std::array<char, AV_ERROR_MAX_STRING_SIZE> errorText { };
            if (av_strerror(errorCode, errorText.data(), errorText.size()) < 0)
            {
                return std::string(operation) + " failed with FFmpeg error " + std::to_string(errorCode);
            }

            return std::string(operation) + " failed: " + errorText.data();
        }

        [[nodiscard]] std::optional<std::chrono::microseconds> Duration(
            const AVFormatContext& format,
            const AVStream& stream
        )
        {
            if (format.duration != AV_NOPTS_VALUE && format.duration >= 0)
            {
                return std::chrono::microseconds(format.duration);
            }

            if (stream.duration != AV_NOPTS_VALUE && stream.duration >= 0)
            {
                return std::chrono::microseconds(
                    av_rescale_q(stream.duration, stream.time_base, AVRational { 1, k_MicrosecondsPerSecond })
                );
            }

            return std::nullopt;
        }

        [[nodiscard]] bool IsRegularVideo(const AVStream& stream)
        {
            return stream.codecpar != nullptr && stream.codecpar->codec_type == AVMEDIA_TYPE_VIDEO
                   && (stream.disposition & AV_DISPOSITION_ATTACHED_PIC) == 0;
        }

        [[nodiscard]] std::expected<ProbedMediaInfo, std::string> InspectStreams(
            const AVFormatContext& format,
            const std::filesystem::path& path
        )
        {
            const AVStream* videoStream = nullptr;
            const AVStream* audioStream = nullptr;
            for (unsigned int index = 0; index < format.nb_streams; ++index)
            {
                const AVStream* stream = format.streams[index];
                if (stream == nullptr || stream->codecpar == nullptr)
                {
                    continue;
                }

                if (videoStream == nullptr && IsRegularVideo(*stream))
                {
                    videoStream = stream;
                }

                if (audioStream == nullptr && stream->codecpar->codec_type == AVMEDIA_TYPE_AUDIO)
                {
                    audioStream = stream;
                }
            }

            if (videoStream != nullptr)
            {
                if (videoStream->codecpar->width <= 0 || videoStream->codecpar->height <= 0)
                {
                    if (LowercaseExtension(path) == "webp")
                    {
                        return std::unexpected(
                            "FFmpeg could not read WebP dimensions; animated WebP may be unsupported by this build"
                        );
                    }

                    return std::unexpected("The video stream has invalid dimensions");
                }

                return ProbedMediaInfo {
                    .kind = EMediaStreamKind::Video,
                    .width = videoStream->codecpar->width,
                    .height = videoStream->codecpar->height,
                    .duration = Duration(format, *videoStream),
                    .hasAudio = audioStream != nullptr,
                };
            }

            if (audioStream != nullptr)
            {
                return ProbedMediaInfo {
                    .kind = EMediaStreamKind::Audio,
                    .width = 0,
                    .height = 0,
                    .duration = Duration(format, *audioStream),
                    .hasAudio = true,
                };
            }

            return std::unexpected("The media file contains no supported audio or video stream");
        }
    } // namespace
} // namespace FoxMediaTerm
