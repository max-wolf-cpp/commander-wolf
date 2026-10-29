#pragma once

#include <expected>
#include <filesystem>
#include <string>
#include <utility>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "Image.h"
#include "ImageRegionRenderer.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"

namespace FoxMediaTerm::Bridge
{
    [[nodiscard]] std::expected<std::pair<RasterImage, FoxMediaTermKind>, std::string> LoadPreview(
        const std::filesystem::path& path,
        ETerminalProtocol protocol,
        TerminalCellPixelSize cellPixelSize,
        ImageCellRegion region
    );
} // namespace FoxMediaTerm::Bridge
