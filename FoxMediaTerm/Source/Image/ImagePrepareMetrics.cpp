#include "ImagePrepareMetrics.h"

namespace FoxMediaTerm
{
    void ImagePrepareMetrics::RecordStage(EImagePrepareStage stage, std::chrono::steady_clock::duration duration)
    {
        const std::size_t index = static_cast<std::size_t>(stage);
        stageNanoseconds[index] += std::chrono::duration_cast<std::chrono::nanoseconds>(duration).count();
        stageRecorded[index] = true;
    }

    void ImagePrepareMetrics::RecordTemporaryBuffer(std::size_t capacity, std::size_t reallocations)
    {
        temporaryBufferCapacity += capacity;
        temporaryBufferReallocations += reallocations;
    }
} // namespace FoxMediaTerm
