#pragma once

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <string_view>

namespace FoxMediaTerm
{
    enum class EImagePrepareStage : std::size_t
    {
        InputConversion,
        BackendEncode,
        PngEncode,
        PngOutputCopy,
        Base64Encode,
        KittyFramingAssembly,
        ITerm2FramingAssembly,
        SixelPaletteQuantization,
        SixelDitherEncode,
        HalfBlockTrueColorCellsAnsiAssembly,
        HalfBlock256CellsAnsiAssembly,
        AsciiCellsAssembly,
        FinalPayloadAssembly,
        PrepareTotal,
        Count,
    };

    struct ImagePrepareMetrics
    {
        std::array<std::int64_t, static_cast<std::size_t>(EImagePrepareStage::Count)> stageNanoseconds { };
        std::array<bool, static_cast<std::size_t>(EImagePrepareStage::Count)> stageRecorded { };
        std::uint64_t inputPixels = 0;
        std::uint64_t inputBytes = 0;
        std::size_t finalPayloadBytes = 0;
        std::size_t finalPayloadCapacity = 0;
        std::size_t finalPayloadReallocations = 0;
        std::size_t temporaryBufferCapacity = 0;
        std::size_t temporaryBufferReallocations = 0;

        void RecordStage(EImagePrepareStage stage, std::chrono::steady_clock::duration duration);
        void RecordTemporaryBuffer(std::size_t capacity, std::size_t reallocations);
    };

    [[nodiscard]] constexpr std::string_view ImagePrepareStageName(EImagePrepareStage stage)
    {
        switch (stage)
        {
            case EImagePrepareStage::InputConversion:
                return "Input conversion / pixel copy";
            case EImagePrepareStage::BackendEncode:
                return "Backend encode / compress";
            case EImagePrepareStage::PngEncode:
                return "PNG filter + DEFLATE + output total";
            case EImagePrepareStage::PngOutputCopy:
                return "PNG output callback / copy";
            case EImagePrepareStage::Base64Encode:
                return "Base64 encode";
            case EImagePrepareStage::KittyFramingAssembly:
                return "Kitty framing + assembly";
            case EImagePrepareStage::ITerm2FramingAssembly:
                return "iTerm2 framing + assembly";
            case EImagePrepareStage::SixelPaletteQuantization:
                return "Sixel palette / quantization";
            case EImagePrepareStage::SixelDitherEncode:
                return "Sixel dithering + encode / framing";
            case EImagePrepareStage::HalfBlockTrueColorCellsAnsiAssembly:
                return "Half-block TC sampling + cells / ANSI assembly";
            case EImagePrepareStage::HalfBlock256CellsAnsiAssembly:
                return "Half-block 256 sampling + cells / ANSI assembly";
            case EImagePrepareStage::AsciiCellsAssembly:
                return "ASCII sampling + cells / assembly";
            case EImagePrepareStage::FinalPayloadAssembly:
                return "Final payload assembly / copy";
            case EImagePrepareStage::PrepareTotal:
                return "Prepare total";
            case EImagePrepareStage::Count:
                break;
        }
        return "Unknown prepare stage";
    }
} // namespace FoxMediaTerm
