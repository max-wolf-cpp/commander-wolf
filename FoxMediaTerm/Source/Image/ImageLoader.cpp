#include "ImageLoader.h"

#include <algorithm>
#include <array>
#include <cctype>
#include <fstream>
#include <limits>
#include <memory>
#include <utility>
#include <vector>

#define STB_IMAGE_IMPLEMENTATION
#include <stb_image.h>

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] std::string PathForMessage(const std::filesystem::path& path)
        {
            const std::u8string text = path.generic_u8string();
            return std::string(reinterpret_cast<const char*>(text.data()), text.size());
        }

        [[nodiscard]] std::string LowercaseExtension(const std::filesystem::path& path)
        {
            std::string extension = path.extension().string();
            std::ranges::transform(extension, extension.begin(), [](unsigned char character) {
                return static_cast<char>(std::tolower(character));
            });

            return extension;
        }

        int ReadImageBytes(void* user, char* data, int size)
        {
            auto& input = *static_cast<std::ifstream*>(user);
            input.read(data, size);
            return static_cast<int>(input.gcount());
        }

        void SkipImageBytes(void* user, int count)
        {
            auto& input = *static_cast<std::ifstream*>(user);
            input.clear();
            input.seekg(count, std::ios::cur);
        }

        int ReachedImageEnd(void* user)
        {
            return static_cast<std::ifstream*>(user)->eof() ? 1 : 0;
        }
    } // namespace

    bool ImageLoader::HasSupportedExtension(const std::filesystem::path& path)
    {
        // Keep this list aligned with the formats advertised by stb_image.
        static constexpr std::array k_SupportedExtensions {
            ".jpg", ".jpeg", ".png", ".tga", ".bmp", ".psd", ".gif", ".hdr", ".pic", ".pnm", ".ppm", ".pgm",
        };
        const std::string extension = LowercaseExtension(path);

        return std::ranges::find(k_SupportedExtensions, extension) != k_SupportedExtensions.end();
    }

    bool ImageLoader::IsSupported(const std::filesystem::path& path)
    {
        return ProbeDimensions(path).has_value();
    }

    std::expected<std::pair<int, int>, std::string> ImageLoader::ProbeDimensions(const std::filesystem::path& path)
    {
        if (!HasSupportedExtension(path))
        {
            return std::unexpected("Unsupported image file extension: " + PathForMessage(path));
        }

        std::ifstream input(path, std::ios::binary);
        if (!input)
        {
            return std::unexpected("Unable to open image: " + PathForMessage(path));
        }

        const stbi_io_callbacks callbacks { ReadImageBytes, SkipImageBytes, ReachedImageEnd };
        int width = 0;
        int height = 0;
        int channels = 0;
        if (stbi_info_from_callbacks(&callbacks, &input, &width, &height, &channels) == 0 || width <= 0 || height <= 0)
        {
            return std::unexpected("Unsupported or invalid image: " + PathForMessage(path));
        }

        return std::pair { width, height };
    }

    std::expected<RgbaImage, std::string> ImageLoader::Load(const std::filesystem::path& path)
    {
        if (!HasSupportedExtension(path))
        {
            return std::unexpected("Unsupported image file extension: " + PathForMessage(path));
        }

        std::ifstream input(path, std::ios::binary | std::ios::ate);
        if (!input)
        {
            return std::unexpected("Unable to open image: " + PathForMessage(path));
        }

        const std::streamoff fileSize = input.tellg();
        if (fileSize <= 0)
        {
            return std::unexpected("Image file is empty: " + PathForMessage(path));
        }

        if (fileSize > std::numeric_limits<int>::max())
        {
            return std::unexpected("Image file is too large to decode: " + PathForMessage(path));
        }

        std::vector<std::uint8_t> encoded(static_cast<std::size_t>(fileSize));
        input.seekg(0, std::ios::beg);
        if (!input.read(reinterpret_cast<char*>(encoded.data()), fileSize))
        {
            return std::unexpected("Unable to read image: " + PathForMessage(path));
        }

        int width = 0;
        int height = 0;
        int sourceChannels = 0;
        stbi_uc* decodedPointer = stbi_load_from_memory(
            encoded.data(),
            static_cast<int>(encoded.size()),
            &width,
            &height,
            &sourceChannels,
            STBI_rgb_alpha
        );
        std::unique_ptr<stbi_uc, decltype(&stbi_image_free)> decoded(decodedPointer, stbi_image_free);

        if (!decoded)
        {
            const char* reason = stbi_failure_reason();
            return std::unexpected(
                "Unable to decode image " + PathForMessage(path) + ": "
                + (reason == nullptr ? "unknown format" : reason)
            );
        }

        constexpr std::size_t k_Channels = 4;
        const std::size_t imageWidth = static_cast<std::size_t>(width);
        const std::size_t imageHeight = static_cast<std::size_t>(height);
        if (imageWidth > std::numeric_limits<std::size_t>::max() / imageHeight / k_Channels)
        {
            return std::unexpected("Decoded image dimensions are too large: " + PathForMessage(path));
        }

        const std::size_t pixelBytes = imageWidth * imageHeight * k_Channels;

        return RgbaImage {
            width,
            height,
            std::vector<std::uint8_t>(decoded.get(), decoded.get() + pixelBytes),
        };
    }
} // namespace FoxMediaTerm
