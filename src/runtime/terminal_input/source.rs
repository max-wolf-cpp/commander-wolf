use std::io;
use std::time::Duration;

#[cfg(not(windows))]
use crossterm::event;
use crossterm::event::Event;

#[cfg(windows)]
mod windows;

#[derive(Default)]
pub(super) struct EventSource {
    #[cfg(windows)]
    console: Option<windows::ConsoleInput>,
}

pub(super) enum ReadOutcome {
    Event(Event),
    #[cfg(windows)]
    Skipped,
    TimedOut,
}

impl EventSource {
    pub(super) fn read(&mut self, timeout: Duration) -> io::Result<ReadOutcome> {
        #[cfg(windows)]
        {
            if self.console.is_none() {
                self.console = Some(windows::ConsoleInput::open()?);
            }
            self.console
                .as_mut()
                .expect("the console input handle was initialized")
                .read(timeout)
        }
        #[cfg(not(windows))]
        {
            if event::poll(timeout)? {
                event::read().map(ReadOutcome::Event)
            } else {
                Ok(ReadOutcome::TimedOut)
            }
        }
    }
}
