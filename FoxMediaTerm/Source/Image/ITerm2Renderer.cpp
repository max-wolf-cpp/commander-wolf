#include "ITerm2Renderer.h"

#include <string>

#include "Base64Encoder.h"
#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    std::expected<std::string, std::string> ITerm2Renderer::Encode(
        const RenderableImage& image,
        ITerm2RenderOptions options,
        ImagePrepareMetrics* metrics,
        PngEncodeFunction pngEncoder
    )
    {
        auto png = pngEncoder(image, metrics);
        if (!png)
        {
            return std::unexpected(png.error());
        }

        const int displayWidth = options.displayWidthPixels > 0 ? options.displayWidthPixels : image.width;
        const int displayHeight = options.displayHeightPixels > 0 ? options.displayHeightPixels : image.height;
        const std::string widthArgument =
            options.displayColumns > 0 ? std::to_string(options.displayColumns) : std::to_string(displayWidth) + "px";
        const std::string heightArgument =
            options.displayRows > 0 ? std::to_string(options.displayRows) : std::to_string(displayHeight) + "px";

        const std::string base64 = Base64Encoder::Encode(*png, metrics);

        const auto framingStart = std::chrono::steady_clock::now();
        std::string escapeStream = "\x1b]1337;File=inline=1;size=" + std::to_string(png->size())
                                   + ";width=" + widthArgument + ";height=" + heightArgument
                                   + ";preserveAspectRatio=" + (options.preserveAspectRatio ? "1" : "0");
        std::size_t reallocations = escapeStream.capacity() > std::string { }.capacity() ? 1 : 0;
        if (options.preserveCursor)
        {
            const std::size_t previousCapacity = escapeStream.capacity();
            escapeStream += ";doNotMoveCursor=1";
            reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;
        }

        std::size_t previousCapacity = escapeStream.capacity();
        escapeStream.push_back(':');
        reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;
        previousCapacity = escapeStream.capacity();
        escapeStream += base64;
        reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;
        previousCapacity = escapeStream.capacity();
        escapeStream += "\x1b\\\n";
        reallocations += escapeStream.capacity() != previousCapacity ? 1 : 0;

        if (metrics != nullptr)
        {
            metrics->RecordStage(
                EImagePrepareStage::ITerm2FramingAssembly,
                std::chrono::steady_clock::now() - framingStart
            );
            metrics->RecordTemporaryBuffer(escapeStream.capacity(), reallocations);
        }

        return escapeStream;
    }
} // namespace FoxMediaTerm
