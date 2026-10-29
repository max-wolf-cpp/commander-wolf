#pragma once

#include <cstddef>
#include <cstdint>
#include <exception>
#include <filesystem>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "ImageRegionRenderer.h"
#include "TerminalGeometry.h"
#include "TerminalProtocol.h"
#include "TerminalProtocolDetector.h"
#include "TerminalSize.h"

// These file-local adapters stay visible to each ABI translation unit so optimized
// polling and raster paths retain inlining and move-only payload publication.
namespace
{
    using FoxMediaTerm::ETerminalProtocol;
    using FoxMediaTerm::ImageCellRegion;
    using FoxMediaTerm::TerminalCellPixelSize;
    using FoxMediaTerm::TerminalCellSize;

    void Reset(FoxMediaTermBuffer* buffer)
    {
        if (buffer != nullptr)
        {
            *buffer = { };
        }
    }

    void Reset(FoxMediaTermError* error)
    {
        if (error != nullptr)
        {
            *error = { };
        }
    }

    void Store(FoxMediaTermBuffer& buffer, std::string payload, bool usesAnsiUtf8)
    {
        if (payload.empty())
        {
            return;
        }

        auto owner = std::make_unique<std::string>(std::move(payload));
        buffer.data = reinterpret_cast<const std::uint8_t*>(owner->data());
        buffer.size = owner->size();
        buffer.context = owner.release();
        buffer.flags = usesAnsiUtf8 ? FOX_MEDIA_TERM_BUFFER_ANSI_UTF8 : 0;
    }

    void Store(FoxMediaTermError* error, std::string message)
    {
        if (error == nullptr)
        {
            return;
        }

        auto owner = std::make_unique<std::string>(std::move(message));
        error->data = owner->data();
        error->size = owner->size();
        error->context = owner.release();
    }

    [[nodiscard]] std::filesystem::path PathFromUtf8(const char* value, std::size_t size)
    {
        if (value == nullptr && size != 0)
        {
            throw std::invalid_argument("The UTF-8 media path is null");
        }

        const std::string_view pathText(value == nullptr ? "" : value, size);
#ifdef _WIN32
        const auto* first = reinterpret_cast<const char8_t*>(pathText.data());
        return std::filesystem::path(std::u8string(first, first + pathText.size()));
#else
        return std::filesystem::path(pathText);
#endif
    }

    [[nodiscard]] ETerminalProtocol ProtocolFromAbi(FoxMediaTermProtocol protocol)
    {
        switch (protocol)
        {
            case FOX_MEDIA_TERM_PROTOCOL_AUTO:
                return FoxMediaTerm::TerminalProtocolDetector::Detect();
            case FOX_MEDIA_TERM_PROTOCOL_ITERM2:
                return ETerminalProtocol::ITerm2;
            case FOX_MEDIA_TERM_PROTOCOL_SIXEL:
                return ETerminalProtocol::Sixel;
            case FOX_MEDIA_TERM_PROTOCOL_KITTY:
                return ETerminalProtocol::Kitty;
            case FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_TRUE_COLOR:
                return ETerminalProtocol::HalfBlockTrueColor;
            case FOX_MEDIA_TERM_PROTOCOL_HALF_BLOCK_256:
                return ETerminalProtocol::HalfBlock256;
            case FOX_MEDIA_TERM_PROTOCOL_ASCII_GRAYSCALE:
                return ETerminalProtocol::AsciiGrayscale;
            default:
                throw std::invalid_argument("The terminal media protocol value is invalid");
        }
    }

    [[nodiscard]] FoxMediaTermProtocol ProtocolToAbi(ETerminalProtocol protocol)
    {
        return static_cast<FoxMediaTermProtocol>(protocol);
    }

    [[nodiscard]] ImageCellRegion RegionFromAbi(FoxMediaTermRegion region)
    {
        return { region.column, region.row, region.columns, region.rows };
    }

    [[nodiscard]] TerminalCellPixelSize CellSize(int width, int height)
    {
        if (width <= 0 || height <= 0)
        {
            throw std::invalid_argument("The terminal cell pixel size must be positive");
        }

        return { width, height, true };
    }

    template <typename TFunction> FoxMediaTermStatus Guard(FoxMediaTermError* error, TFunction&& function)
    {
        Reset(error);

        try
        {
            return std::forward<TFunction>(function)();
        }
        catch (const std::exception& exception)
        {
            Store(error, exception.what());
        }
        catch (...)
        {
            Store(error, "The media library failed with an unknown exception");
        }

        return FOX_MEDIA_TERM_STATUS_ERROR;
    }
} // namespace
