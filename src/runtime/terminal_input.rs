use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::Event;

use crate::terminal_appearance::{
    AppearanceReport, TerminalAppearance, parse_response, parse_scheme_response,
};
use crate::theme::Rgb;

use self::replies::ReplyFilter;
use self::source::{EventSource, ReadOutcome};

mod replies;
mod source;

const MAX_EVENTS_PER_POLL: usize = 128;

#[derive(Default)]
pub(super) struct TerminalInput {
    source: EventSource,
    replies: ReplyFilter,
    ready: VecDeque<Event>,
    appearance: TerminalAppearance,
    palette: [Option<Rgb>; 16],
    appearance_changed: bool,
}

impl TerminalInput {
    pub(super) fn poll(&mut self, timeout: Duration) -> Result<Option<Event>, String> {
        let started = Instant::now();
        if let Some(event) = self.ready.pop_front() {
            return Ok(Some(event));
        }

        for _ in 0..MAX_EVENTS_PER_POLL {
            let remaining = timeout.saturating_sub(started.elapsed());
            match self
                .source
                .read(remaining)
                .map_err(|error| format!("cannot read terminal input: {error}"))?
            {
                ReadOutcome::Event(event) => self.push(event, Instant::now()),
                #[cfg(windows)]
                ReadOutcome::Skipped => {}
                ReadOutcome::TimedOut => {
                    // Expire only after the input queue is idle. A slow frame must
                    // not split a reply whose remaining events are already queued.
                    self.replies.expire(Instant::now(), &mut self.ready);
                    break;
                }
            }
            if let Some(event) = self.ready.pop_front() {
                return Ok(Some(event));
            }
            if !timeout.is_zero() && started.elapsed() >= timeout {
                break;
            }
        }

        Ok(self.ready.pop_front())
    }

    pub(super) fn begin_appearance_query(&mut self) {
        self.palette.fill(None);
    }

    pub(super) fn take_appearance(&mut self) -> Option<TerminalAppearance> {
        std::mem::take(&mut self.appearance_changed).then_some(self.appearance)
    }

    fn push(&mut self, event: Event, now: Instant) {
        let Some(reply) = self.replies.push(event, now, &mut self.ready) else {
            return;
        };
        let Some(report) = parse_response(&reply).or_else(|| parse_scheme_response(&reply)) else {
            return;
        };

        let previous = self.appearance;
        match report {
            AppearanceReport::Background(color) => self.appearance.background = Some(color),
            AppearanceReport::Scheme(scheme) => self.appearance.scheme = scheme,
            AppearanceReport::PaletteColor(index, color) => {
                self.palette[usize::from(index)] = Some(color);
                if self.palette.iter().all(Option::is_some) {
                    self.appearance.palette = Some(
                        self.palette
                            .map(|color| color.expect("all palette colors present")),
                    );
                }
            }
        }
        self.appearance_changed |= self.appearance != previous;
    }
}

#[cfg(test)]
mod tests;
