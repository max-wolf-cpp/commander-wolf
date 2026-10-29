#include "PreparedVideoFrameBuffer.h"

#include <algorithm>
#include <chrono>
#include <tuple>
#include <utility>

#include "VideoPlaybackTelemetry.h"

namespace FoxMediaTerm
{
    namespace
    {
        [[nodiscard]] auto FrameOrder(const PreparedVideoFrame& frame)
        {
            return std::tie(frame.timestamp, frame.index);
        }
    } // namespace

    PreparedVideoFrameBuffer::PreparedVideoFrameBuffer(VideoPlaybackTelemetry* telemetry)
        : m_Telemetry(telemetry)
    {
    }

    std::optional<std::size_t> PreparedVideoFrameBuffer::AcquirePreparationSlot(std::stop_token stopToken)
    {
        const auto waitStart = std::chrono::steady_clock::now();
        std::unique_lock lock(m_Mutex);
        if (stopToken.stop_requested())
        {
            lock.unlock();
            if (m_Telemetry != nullptr)
            {
                m_Telemetry->RecordProducerWait(std::chrono::steady_clock::now() - waitStart);
            }

            return std::nullopt;
        }

        const bool acquired = m_SlotAvailable.wait(lock, stopToken, [this] {
            return FindFreeSlot().has_value() || m_Finished || m_Error.has_value();
        });
        if (!acquired || stopToken.stop_requested() || m_Finished || m_Error)
        {
            lock.unlock();
            if (m_Telemetry != nullptr)
            {
                m_Telemetry->RecordProducerWait(std::chrono::steady_clock::now() - waitStart);
            }

            return std::nullopt;
        }

        const std::size_t slot = *FindFreeSlot();
        m_Slots[slot].state = ESlotState::Preparing;
        lock.unlock();
        if (m_Telemetry != nullptr)
        {
            m_Telemetry->RecordProducerWait(std::chrono::steady_clock::now() - waitStart);
        }

        return slot;
    }

    void PreparedVideoFrameBuffer::Publish(std::size_t slot, PreparedVideoFrame frame)
    {
        std::size_t readyDepth = 0;
        {
            std::lock_guard lock(m_Mutex);
            if (m_Slots.at(slot).state != ESlotState::Preparing)
            {
                return;
            }

            m_Slots[slot].frame = std::move(frame);
            m_Slots[slot].state = ESlotState::Ready;
            readyDepth = static_cast<std::size_t>(std::ranges::count_if(m_Slots, [](const Slot& candidate) {
                return candidate.state == ESlotState::Ready;
            }));
        }

        if (m_Telemetry != nullptr)
        {
            m_Telemetry->ObserveReadyDepth(readyDepth);
        }

        m_FrameAvailable.notify_one();
    }

    void PreparedVideoFrameBuffer::CancelPreparation(std::size_t slot)
    {
        {
            std::lock_guard lock(m_Mutex);
            if (m_Slots.at(slot).state == ESlotState::Preparing)
            {
                m_Slots[slot].frame = { };
                m_Slots[slot].state = ESlotState::Free;
            }
        }
        m_SlotAvailable.notify_all();
    }

    void PreparedVideoFrameBuffer::FailPreparation(std::size_t slot, std::string error)
    {
        {
            std::lock_guard lock(m_Mutex);
            if (m_Slots.at(slot).state == ESlotState::Preparing)
            {
                m_Slots[slot].frame = { };
                m_Slots[slot].state = ESlotState::Free;
            }

            if (!m_Error)
            {
                m_Error = std::move(error);
            }
            m_Finished = true;
        }
        m_FrameAvailable.notify_all();
        m_SlotAvailable.notify_all();
    }

    void PreparedVideoFrameBuffer::Finish()
    {
        {
            std::lock_guard lock(m_Mutex);
            m_Finished = true;
        }
        m_FrameAvailable.notify_all();
        m_SlotAvailable.notify_all();
    }

    std::expected<PreparedFrameWaitResult, std::string> PreparedVideoFrameBuffer::WaitForReady(
        std::chrono::milliseconds timeout
    )
    {
        std::unique_lock lock(m_Mutex);
        m_FrameAvailable.wait_for(lock, timeout, [this] {
            return FindOldestReadySlot().has_value() || m_Finished || m_Error.has_value();
        });

        if (m_Error)
        {
            return std::unexpected(*m_Error);
        }

        if (const auto slot = FindOldestReadySlot())
        {
            m_Slots[*slot].state = ESlotState::Writing;
            return PreparedFrameWaitResult {
                .status = EPreparedFrameWaitStatus::Ready,
                .slot = *slot,
            };
        }

        return PreparedFrameWaitResult {
            .status = m_Finished ? EPreparedFrameWaitStatus::Finished : EPreparedFrameWaitStatus::Timeout,
        };
    }

    PreparedFrameSelection PreparedVideoFrameBuffer::SelectNewestDue(
        std::size_t writingSlot,
        std::chrono::microseconds latestDueTimestamp
    )
    {
        std::lock_guard lock(m_Mutex);
        Slot& current = m_Slots.at(writingSlot);
        if (current.state != ESlotState::Writing)
        {
            return { .slot = writingSlot };
        }

        std::size_t selectedSlot = writingSlot;
        for (std::size_t slot = 0; slot < m_Slots.size(); ++slot)
        {
            const Slot& candidate = m_Slots[slot];
            if (candidate.state == ESlotState::Ready && candidate.frame.timestamp <= latestDueTimestamp
                && FrameOrder(m_Slots[selectedSlot].frame) < FrameOrder(candidate.frame))
            {
                selectedSlot = slot;
            }
        }

        std::size_t dropped = 0;
        for (std::size_t slot = 0; slot < m_Slots.size(); ++slot)
        {
            Slot& candidate = m_Slots[slot];
            const bool isObsoleteReady = candidate.state == ESlotState::Ready
                                         && candidate.frame.timestamp <= latestDueTimestamp && slot != selectedSlot;
            const bool isReplacedWriting = slot == writingSlot && slot != selectedSlot;
            if (isObsoleteReady || isReplacedWriting)
            {
                candidate.frame = { };
                candidate.state = ESlotState::Free;
                ++dropped;
            }
        }

        m_Slots[selectedSlot].state = ESlotState::Writing;
        m_SlotAvailable.notify_all();

        return {
            .slot = selectedSlot,
            .dropped = dropped,
        };
    }

    const PreparedVideoFrame& PreparedVideoFrameBuffer::Frame(std::size_t slot) const
    {
        return m_Slots.at(slot).frame;
    }

    PreparedVideoFrame PreparedVideoFrameBuffer::Take(std::size_t slot)
    {
        PreparedVideoFrame frame;
        {
            std::lock_guard lock(m_Mutex);
            Slot& selected = m_Slots.at(slot);
            if (selected.state == ESlotState::Writing)
            {
                frame = std::move(selected.frame);
                selected.frame = { };
                selected.state = ESlotState::Free;
            }
        }
        m_SlotAvailable.notify_all();

        return frame;
    }

    void PreparedVideoFrameBuffer::Release(std::size_t slot)
    {
        {
            std::lock_guard lock(m_Mutex);
            Slot& released = m_Slots.at(slot);
            if (released.state == ESlotState::Writing)
            {
                released.frame = { };
                released.state = ESlotState::Free;
            }
        }
        m_SlotAvailable.notify_all();
    }

    std::size_t PreparedVideoFrameBuffer::ReadyCount() const
    {
        std::lock_guard lock(m_Mutex);
        return static_cast<std::size_t>(std::ranges::count_if(m_Slots, [](const Slot& slot) {
            return slot.state == ESlotState::Ready;
        }));
    }

    std::optional<std::size_t> PreparedVideoFrameBuffer::FindFreeSlot() const
    {
        for (std::size_t slot = 0; slot < m_Slots.size(); ++slot)
        {
            if (m_Slots[slot].state == ESlotState::Free)
            {
                return slot;
            }
        }

        return std::nullopt;
    }

    std::optional<std::size_t> PreparedVideoFrameBuffer::FindOldestReadySlot() const
    {
        std::optional<std::size_t> oldest;
        for (std::size_t slot = 0; slot < m_Slots.size(); ++slot)
        {
            if (m_Slots[slot].state == ESlotState::Ready
                && (!oldest || FrameOrder(m_Slots[slot].frame) < FrameOrder(m_Slots[*oldest].frame)))
            {
                oldest = slot;
            }
        }

        return oldest;
    }
} // namespace FoxMediaTerm
