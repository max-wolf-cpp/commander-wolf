#include <chrono>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <string_view>

#include <FoxMediaTerm/FoxMediaTerm.h>

#include "Bridge/AbiBoundary.h"
#include "TerminalGeometry.h"
#include "TerminalProbe.h"
#include "TerminalProtocolDetector.h"
#include "TerminalSize.h"

extern "C"
{
    FoxMediaTermStatus FoxMediaTermQueryTerminal(
        const uint8_t* request,
        size_t requestSize,
        uint32_t timeoutMilliseconds,
        FoxMediaTermBuffer* response,
        FoxMediaTermError* error
    )
    {
        Reset(response);

        return Guard(error, [=] {
            if (response == nullptr || request == nullptr || requestSize == 0 || timeoutMilliseconds > 500)
            {
                throw std::invalid_argument("Invalid terminal query arguments");
            }

            const auto answer = FoxMediaTerm::TerminalProbe::Query(
                std::string_view(reinterpret_cast<const char*>(request), requestSize),
                std::chrono::milliseconds(timeoutMilliseconds)
            );
            if (answer && !answer->empty())
            {
                Store(*response, *answer, false);
            }

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }

    FoxMediaTermStatus FoxMediaTermDetectEnvironment(
        int32_t terminalColumns,
        int32_t terminalRows,
        FoxMediaTermEnvironment* environment,
        FoxMediaTermError* error
    )
    {
        return Guard(error, [=] {
            if (environment == nullptr)
            {
                throw std::invalid_argument("The media environment output is null");
            }

            const TerminalCellSize terminalSize { terminalColumns, terminalRows };
            const ETerminalProtocol protocol = FoxMediaTerm::TerminalProtocolDetector::Detect();
            const TerminalCellPixelSize cellSize = FoxMediaTerm::TerminalGeometry::Detect(terminalSize);

            *environment = {
                .protocol = ProtocolToAbi(protocol),
                .cellPixelWidth = cellSize.width,
                .cellPixelHeight = cellSize.height,
                .exactCellPixelSize = cellSize.exact ? 1U : 0U,
            };

            return FOX_MEDIA_TERM_STATUS_OK;
        });
    }

    void FoxMediaTermReleaseBuffer(FoxMediaTermBuffer* buffer)
    {
        if (buffer == nullptr)
        {
            return;
        }

        delete static_cast<std::string*>(buffer->context);
        *buffer = { };
    }

    void FoxMediaTermReleaseError(FoxMediaTermError* error)
    {
        if (error == nullptr)
        {
            return;
        }

        delete static_cast<std::string*>(error->context);
        *error = { };
    }
}
