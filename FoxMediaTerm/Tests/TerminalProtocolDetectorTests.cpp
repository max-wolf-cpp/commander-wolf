#include <string>
#include <string_view>

#include <gtest/gtest.h>

#include "TerminalGeometry.h"
#include "TerminalProtocolDetector.h"

namespace
{
    TEST(TerminalProtocolDetector, ParsesActiveProtocolAndTextCapabilityResponses)
    {
        constexpr std::string_view k_Response = "\x1b_Gi=31;OK\x1b\\"
                                                "\x1b]1337;Capabilities=T3UFSx\x1b\\"
                                                "\x1b[?61;4;6;7c"
                                                "\x1bP>|iTerm2 3.5.0\x1b\\";

        const FoxMediaTerm::TerminalCapabilities capabilities =
            FoxMediaTerm::TerminalProtocolDetector::ParseProbeResponse(k_Response);

        EXPECT_TRUE(capabilities.kittyGraphics);
        EXPECT_TRUE(capabilities.iTerm2Graphics);
        EXPECT_TRUE(capabilities.sixelGraphics);
        EXPECT_TRUE(capabilities.unicode);
        EXPECT_TRUE(capabilities.trueColor);
        EXPECT_TRUE(capabilities.color256);
    }

    TEST(TerminalProtocolDetector, TreatsAnyKittyQueryReplyAsProtocolSupport)
    {
        constexpr std::string_view k_Response = "\x1b_Gi=31;EINVAL:unsupported payload\x1b\\";

        const FoxMediaTerm::TerminalCapabilities capabilities =
            FoxMediaTerm::TerminalProtocolDetector::ParseProbeResponse(k_Response);

        EXPECT_TRUE(capabilities.kittyGraphics);
    }

    TEST(TerminalProtocolDetector, RecognizesWezTermVersionRepliesAsITerm2Support)
    {
        constexpr std::string_view k_Response = "\x1bP>|WezTerm 20240203-110809\x1b\\";

        const FoxMediaTerm::TerminalCapabilities capabilities =
            FoxMediaTerm::TerminalProtocolDetector::ParseProbeResponse(k_Response);

        EXPECT_TRUE(capabilities.iTerm2Graphics);
    }

    TEST(TerminalProtocolDetector, SelectsBackendsInTheRequiredFallbackOrder)
    {
        FoxMediaTerm::TerminalCapabilities capabilities {
            .kittyGraphics = true,
            .iTerm2Graphics = true,
            .sixelGraphics = true,
            .unicode = true,
            .trueColor = true,
            .color256 = true,
        };

        EXPECT_EQ(FoxMediaTerm::TerminalProtocolDetector::Select(capabilities), FoxMediaTerm::ETerminalProtocol::Kitty);
        capabilities.kittyGraphics = false;
        EXPECT_EQ(
            FoxMediaTerm::TerminalProtocolDetector::Select(capabilities),
            FoxMediaTerm::ETerminalProtocol::ITerm2
        );
        capabilities.iTerm2Graphics = false;
        EXPECT_EQ(FoxMediaTerm::TerminalProtocolDetector::Select(capabilities), FoxMediaTerm::ETerminalProtocol::Sixel);
        capabilities.sixelGraphics = false;
        EXPECT_EQ(
            FoxMediaTerm::TerminalProtocolDetector::Select(capabilities),
            FoxMediaTerm::ETerminalProtocol::HalfBlockTrueColor
        );
        capabilities.trueColor = false;
        EXPECT_EQ(
            FoxMediaTerm::TerminalProtocolDetector::Select(capabilities),
            FoxMediaTerm::ETerminalProtocol::HalfBlock256
        );
        capabilities.color256 = false;
        EXPECT_EQ(
            FoxMediaTerm::TerminalProtocolDetector::Select(capabilities),
            FoxMediaTerm::ETerminalProtocol::AsciiGrayscale
        );
    }

    TEST(TerminalProtocolDetector, RequiresUnicodeForHalfBlockFallbacks)
    {
        const FoxMediaTerm::TerminalCapabilities capabilities {
            .unicode = false,
            .trueColor = true,
            .color256 = true,
        };

        EXPECT_EQ(
            FoxMediaTerm::TerminalProtocolDetector::Select(capabilities),
            FoxMediaTerm::ETerminalProtocol::AsciiGrayscale
        );
    }

    TEST(TerminalGeometry, ParsesDirectCellPixelReports)
    {
        const auto geometry = FoxMediaTerm::TerminalGeometry::ParseResponse("\x1b[6;20;10t", { 160, 50 });

        ASSERT_TRUE(geometry.has_value());
        EXPECT_EQ(geometry->width, 10);
        EXPECT_EQ(geometry->height, 20);
        EXPECT_TRUE(geometry->exact);
    }

    TEST(TerminalGeometry, DerivesCellPixelsFromWindowPixelReports)
    {
        const auto geometry = FoxMediaTerm::TerminalGeometry::ParseResponse("\x1b[4;1000;1600t", { 160, 50 });

        ASSERT_TRUE(geometry.has_value());
        EXPECT_EQ(geometry->width, 10);
        EXPECT_EQ(geometry->height, 20);
        EXPECT_TRUE(geometry->exact);
    }
} // namespace
