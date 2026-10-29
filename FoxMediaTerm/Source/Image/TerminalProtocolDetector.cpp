#include "TerminalProtocolDetector.h"

#include <cctype>
#include <chrono>
#include <cstdlib>
#include <string>
#include <string_view>

#include "TerminalProbe.h"

namespace FoxMediaTerm
{
    namespace
    {
        constexpr std::chrono::milliseconds k_ProbeTimeout { 250 };
        // Kitty requires its query action to be followed by DA. The same transaction also requests
        // iTerm2 feature reporting, XTVERSION, and the DA flag used to advertise Sixel.
        constexpr std::string_view k_ProbeRequest = "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\"
                                                    "\x1b[c"
                                                    "\x1b]1337;Capabilities\x1b\\"
                                                    "\x1b[>q";

        [[nodiscard]] std::string EnvironmentValue(const char* name)
        {
#ifdef _WIN32
            char* value = nullptr;
            std::size_t valueSize = 0;
            if (_dupenv_s(&value, &valueSize, name) != 0 || value == nullptr)
            {
                return { };
            }

            std::string result(value);
            std::free(value);
            return result;
#else
            const char* value = std::getenv(name);
            return value == nullptr ? std::string { } : std::string(value);
#endif
        }

        [[nodiscard]] bool IsUppercase(char character)
        {
            return std::isupper(static_cast<unsigned char>(character)) != 0;
        }

        [[nodiscard]] bool IsLowercase(char character)
        {
            return std::islower(static_cast<unsigned char>(character)) != 0;
        }

        [[nodiscard]] bool IsDigit(char character)
        {
            return std::isdigit(static_cast<unsigned char>(character)) != 0;
        }

        void ApplyFeatureString(std::string_view features, TerminalCapabilities& capabilities)
        {
            std::size_t offset = 0;
            while (offset < features.size() && IsUppercase(features[offset]))
            {
                const std::size_t codeStart = offset++;
                while (offset < features.size() && IsLowercase(features[offset]))
                {
                    ++offset;
                }
                const std::size_t codeEnd = offset;

                unsigned int value = 0;
                bool hasValue = false;
                while (offset < features.size() && IsDigit(features[offset]))
                {
                    hasValue = true;
                    value = value * 10 + static_cast<unsigned int>(features[offset] - '0');
                    ++offset;
                }

                const std::string_view code = features.substr(codeStart, codeEnd - codeStart);
                if (code == "F")
                {
                    capabilities.iTerm2Graphics = true;
                }
                else if (code == "Sx")
                {
                    capabilities.sixelGraphics = true;
                }
                else if (code == "U")
                {
                    capabilities.unicode = true;
                }
                else if (code == "T" && hasValue && value > 0)
                {
                    capabilities.trueColor = true;
                    capabilities.color256 = true;
                }
            }
        }

        void ApplyCapabilityReports(std::string_view response, TerminalCapabilities& capabilities)
        {
            constexpr std::string_view k_ReportPrefix = "1337;Capabilities=";
            std::size_t searchOffset = 0;
            while (true)
            {
                const std::size_t report = response.find(k_ReportPrefix, searchOffset);
                if (report == std::string_view::npos)
                {
                    return;
                }

                const std::size_t featuresStart = report + k_ReportPrefix.size();
                std::size_t featuresEnd = featuresStart;
                while (featuresEnd < response.size()
                       && std::isalnum(static_cast<unsigned char>(response[featuresEnd])) != 0)
                {
                    ++featuresEnd;
                }
                ApplyFeatureString(response.substr(featuresStart, featuresEnd - featuresStart), capabilities);
                searchOffset = featuresEnd;
            }
        }

        [[nodiscard]] bool ContainsSixelDeviceAttribute(std::string_view response)
        {
            std::size_t searchOffset = 0;
            while (true)
            {
                const std::size_t report = response.find("\x1b[?", searchOffset);
                if (report == std::string_view::npos)
                {
                    return false;
                }

                const std::size_t end = response.find('c', report + 3);
                if (end == std::string_view::npos)
                {
                    return false;
                }

                std::size_t parameterStart = report + 3;
                while (parameterStart < end)
                {
                    const std::size_t parameterEnd = response.find(';', parameterStart);
                    const std::size_t boundedEnd =
                        parameterEnd == std::string_view::npos || parameterEnd > end ? end : parameterEnd;
                    if (response.substr(parameterStart, boundedEnd - parameterStart) == "4")
                    {
                        return true;
                    }

                    parameterStart = boundedEnd + 1;
                }

                searchOffset = end + 1;
            }
        }

        [[nodiscard]] bool ContainsTerminalName(std::string_view response, std::string_view name)
        {
            constexpr std::string_view k_VersionPrefix = "\x1bP>|";
            std::size_t searchOffset = 0;
            while (true)
            {
                const std::size_t version = response.find(k_VersionPrefix, searchOffset);
                if (version == std::string_view::npos)
                {
                    return false;
                }

                const std::size_t nameOffset = version + k_VersionPrefix.size();
                if (response.substr(nameOffset).starts_with(name))
                {
                    return true;
                }

                searchOffset = nameOffset;
            }
        }

        void Merge(TerminalCapabilities& destination, const TerminalCapabilities& source)
        {
            destination.kittyGraphics = destination.kittyGraphics || source.kittyGraphics;
            destination.iTerm2Graphics = destination.iTerm2Graphics || source.iTerm2Graphics;
            destination.sixelGraphics = destination.sixelGraphics || source.sixelGraphics;
            destination.unicode = destination.unicode || source.unicode;
            destination.trueColor = destination.trueColor || source.trueColor;
            destination.color256 = destination.color256 || source.color256 || destination.trueColor;
        }
    } // namespace

    ETerminalProtocol TerminalProtocolDetector::Detect()
    {
        return Select(Probe());
    }

    TerminalCapabilities TerminalProtocolDetector::Probe()
    {
        const TerminalTextSupport textSupport = TerminalProbe::DetectTextSupport();
        TerminalCapabilities capabilities {
            .interactive = textSupport.interactive,
            .unicode = textSupport.unicode,
            .trueColor = textSupport.trueColor,
            .color256 = textSupport.color256,
        };
        if (!textSupport.interactive)
        {
            return capabilities;
        }

        ApplyFeatureString(EnvironmentValue("TERM_FEATURES"), capabilities);

        if (auto response = TerminalProbe::Query(k_ProbeRequest, k_ProbeTimeout))
        {
            Merge(capabilities, ParseProbeResponse(*response));
        }

        return capabilities;
    }

    TerminalCapabilities TerminalProtocolDetector::ParseProbeResponse(std::string_view response)
    {
        TerminalCapabilities capabilities;
        capabilities.kittyGraphics = response.find("\x1b_Gi=31;") != std::string_view::npos;
        capabilities.iTerm2Graphics =
            ContainsTerminalName(response, "iTerm2") || ContainsTerminalName(response, "WezTerm");
        capabilities.sixelGraphics = ContainsSixelDeviceAttribute(response);
        ApplyCapabilityReports(response, capabilities);

        return capabilities;
    }

    ETerminalProtocol TerminalProtocolDetector::Select(const TerminalCapabilities& capabilities)
    {
        if (capabilities.kittyGraphics)
        {
            return ETerminalProtocol::Kitty;
        }

        if (capabilities.iTerm2Graphics)
        {
            return ETerminalProtocol::ITerm2;
        }

        if (capabilities.sixelGraphics)
        {
            return ETerminalProtocol::Sixel;
        }

        if (capabilities.unicode && capabilities.trueColor)
        {
            return ETerminalProtocol::HalfBlockTrueColor;
        }

        if (capabilities.unicode && capabilities.color256)
        {
            return ETerminalProtocol::HalfBlock256;
        }

        return ETerminalProtocol::AsciiGrayscale;
    }
} // namespace FoxMediaTerm
