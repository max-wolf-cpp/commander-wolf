use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::io::AsRawHandle;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use windows_sys::Win32::Foundation::{WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Console::{
    INPUT_RECORD, KEY_EVENT, LEFT_ALT_PRESSED, LEFT_CTRL_PRESSED, PeekConsoleInputW,
    RIGHT_ALT_PRESSED, RIGHT_CTRL_PRESSED, ReadConsoleInputW, SHIFT_PRESSED,
};
use windows_sys::Win32::System::Threading::WaitForSingleObject;

use super::ReadOutcome;

pub(super) struct ConsoleInput {
    handle: File,
}

impl ConsoleInput {
    pub(super) fn open() -> io::Result<Self> {
        // Match Crossterm's console device even when stdin was redirected.
        let handle = OpenOptions::new().read(true).write(true).open("CONIN$")?;
        Ok(Self { handle })
    }

    pub(super) fn read(&mut self, timeout: Duration) -> io::Result<ReadOutcome> {
        let handle = self.handle.as_raw_handle();
        let milliseconds = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
        let milliseconds = milliseconds.min(u32::MAX - 1);
        match unsafe { WaitForSingleObject(handle, milliseconds) } {
            WAIT_TIMEOUT => return Ok(ReadOutcome::TimedOut),
            WAIT_FAILED => return Err(io::Error::last_os_error()),
            WAIT_OBJECT_0 => {}
            _ => return Err(io::Error::other("unexpected console input wait result")),
        }

        let mut record = INPUT_RECORD::default();
        let mut count = 0;
        if unsafe { PeekConsoleInputW(handle, &mut record, 1, &mut count) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Ok(ReadOutcome::Skipped);
        }

        if control_event(&record).is_some() {
            // Reply characters carry VK=0. Crossterm 0.29 sends C0 characters
            // through ToUnicodeEx(VK=0), losing ESC/BEL and leaving the reply's
            // printable payload to be interpreted as ordinary keyboard input.
            if unsafe { ReadConsoleInputW(handle, &mut record, 1, &mut count) } == 0 {
                return Err(io::Error::last_os_error());
            }
            if count == 0 {
                return Ok(ReadOutcome::Skipped);
            }
            let event = control_event(&record).ok_or_else(|| {
                io::Error::other("console input changed while reading a terminal reply control")
            })?;
            return Ok(ReadOutcome::Event(event));
        }

        // With a zero timeout, Crossterm's Windows source processes one record.
        // Never let it wait here: an ignored modifier record could otherwise be
        // followed by an ESC reply record before control returns to this adapter.
        if event::poll(Duration::ZERO)? {
            event::read().map(ReadOutcome::Event)
        } else {
            Ok(ReadOutcome::Skipped)
        }
    }
}

fn control_event(record: &INPUT_RECORD) -> Option<Event> {
    if record.EventType != KEY_EVENT as u16 {
        return None;
    }
    // EventType selects the initialized union member. Console reply controls
    // have no virtual key; physical keys retain Crossterm's normal decoding.
    let key = unsafe { record.Event.KeyEvent };
    if key.wVirtualKeyCode != 0 {
        return None;
    }
    let code = match unsafe { key.uChar.UnicodeChar } {
        0x1b => KeyCode::Esc,
        0x07 => KeyCode::Char('\x07'),
        _ => return None,
    };

    let mut modifiers = KeyModifiers::NONE;
    for (flags, modifier) in [
        (SHIFT_PRESSED, KeyModifiers::SHIFT),
        (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED, KeyModifiers::ALT),
        (
            LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED,
            KeyModifiers::CONTROL,
        ),
    ] {
        if key.dwControlKeyState & flags != 0 {
            modifiers.insert(modifier);
        }
    }
    let kind = if key.bKeyDown != 0 {
        KeyEventKind::Press
    } else {
        KeyEventKind::Release
    };

    Some(Event::Key(KeyEvent::new_with_kind(code, modifiers, kind)))
}

#[cfg(test)]
mod tests;
