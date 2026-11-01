use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const PREFIX_TIMEOUT: Duration = Duration::from_millis(50);
const REPLY_IDLE_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_REPLY_BYTES: usize = 1024;
const MAX_PENDING_KEYS: usize = 2048;
const STRING_PREFIXES: &[&[u8]] = &[
    b"\x1b]4;",
    b"\x1b]11;",
    b"\x1b]1337;Capabilities",
    b"\x1b_G",
    b"\x1bP>|",
];

#[derive(Default)]
pub(super) struct ReplyFilter {
    bytes: Vec<u8>,
    keys: Vec<KeyEvent>,
    last_byte: Option<Instant>,
    trailing_escape: Option<KeyEvent>,
    confirmed: bool,
    overflowed: bool,
}

#[derive(Clone, Copy)]
enum Sequence {
    Prefix,
    Reply,
    Complete,
    Unrelated,
}

impl ReplyFilter {
    pub(super) fn push(
        &mut self,
        event: Event,
        now: Instant,
        ready: &mut VecDeque<Event>,
    ) -> Option<Vec<u8>> {
        let Event::Key(key) = event else {
            // Focus/resize notifications are independent of the reply bytes.
            // They must not expose a partially received escape introducer.
            ready.push_back(event);
            return None;
        };

        if key.kind != KeyEventKind::Press {
            if !self.bytes.is_empty() && !self.confirmed && self.keys.len() < MAX_PENDING_KEYS {
                self.keys.push(key);
            } else {
                ready.push_back(Event::Key(key));
            }
            return None;
        }

        let Some((encoded, length)) = key_bytes(key) else {
            if !self.confirmed {
                self.release(ready);
            }
            ready.push_back(Event::Key(key));
            return None;
        };
        let encoded = &encoded[..length];

        // Crossterm reports OSC/APC/DCS introducers as Alt+character on Unix
        // and as separate character events on Windows. Retain the original
        // events until a known reply prefix is confirmed.
        if self.bytes.len() > 1 && encoded[0] == b'\x1b' && encoded != b"\x1b\\" && length > 1 {
            self.release(ready);
        } else if self.bytes.len() > 1
            && encoded != b"\\"
            && let Some(escape) = self.trailing_escape.take()
        {
            self.release(ready);
            let _ = self.push(Event::Key(escape), now, ready);
        }

        if self.bytes.is_empty() && !self.overflowed && encoded[0] != b'\x1b' {
            ready.push_back(Event::Key(key));
            return None;
        }

        self.last_byte = Some(now);
        self.trailing_escape = (encoded == b"\x1b").then_some(key);
        if !self.overflowed {
            self.keys.push(key);
        }
        self.bytes.extend_from_slice(encoded);

        if self.overflowed {
            if self.bytes.ends_with(b"\x07") || self.bytes.ends_with(b"\x1b\\") {
                self.clear();
            } else {
                let last = *self.bytes.last().expect("an encoded key was appended");
                self.bytes.clear();
                self.bytes.push(last);
            }
            return None;
        }

        match classify(&self.bytes) {
            Sequence::Prefix => {}
            Sequence::Reply => self.confirmed = true,
            Sequence::Complete => {
                let reply = std::mem::take(&mut self.bytes);
                self.clear();
                return Some(reply);
            }
            Sequence::Unrelated => self.release(ready),
        }

        if self.bytes.len() > MAX_REPLY_BYTES || self.keys.len() >= MAX_PENDING_KEYS {
            if self.confirmed {
                // Discard the rest of an oversized control string through its
                // terminator without allocating or exposing its payload as keys.
                self.overflowed = true;
                self.keys.clear();
                self.bytes.clear();
            } else {
                self.release(ready);
            }
        }

        None
    }

    pub(super) fn expire(&mut self, now: Instant, ready: &mut VecDeque<Event>) {
        let timeout = if self.confirmed {
            REPLY_IDLE_TIMEOUT
        } else {
            PREFIX_TIMEOUT
        };
        if self
            .last_byte
            .is_some_and(|last| now.duration_since(last) >= timeout)
        {
            self.release(ready);
        }
    }

    fn release(&mut self, ready: &mut VecDeque<Event>) {
        if !self.confirmed {
            ready.extend(self.keys.drain(..).map(Event::Key));
        }
        self.clear();
    }

    fn clear(&mut self) {
        self.bytes.clear();
        self.keys.clear();
        self.last_byte = None;
        self.trailing_escape = None;
        self.confirmed = false;
        self.overflowed = false;
    }
}

fn key_bytes(key: KeyEvent) -> Option<([u8; 2], usize)> {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => Some(([b'\x1b', 0], 1)),
        (KeyCode::Char('g'), KeyModifiers::CONTROL) => Some(([b'\x07', 0], 1)),
        (KeyCode::Char(character), modifiers) if character.is_ascii() => {
            let modifiers = modifiers - KeyModifiers::SHIFT;
            if modifiers.is_empty() {
                Some(([character as u8, 0], 1))
            } else if modifiers == KeyModifiers::ALT {
                Some(([b'\x1b', character as u8], 2))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn classify(bytes: &[u8]) -> Sequence {
    for prefix in STRING_PREFIXES {
        if bytes.len() < prefix.len() && prefix.starts_with(bytes) {
            return Sequence::Prefix;
        }
        if bytes.starts_with(prefix) {
            return if bytes.ends_with(b"\x1b\\")
                || prefix.get(1) == Some(&b']') && bytes.ends_with(b"\x07")
            {
                Sequence::Complete
            } else {
                Sequence::Reply
            };
        }
    }

    if b"\x1b[".starts_with(bytes) {
        return Sequence::Prefix;
    }
    let Some(body) = bytes.strip_prefix(b"\x1b[") else {
        return Sequence::Unrelated;
    };
    let Some((&last, parameters)) = body.split_last() else {
        return Sequence::Prefix;
    };
    if (b'@'..=b'~').contains(&last) {
        let known = match last {
            b'n' => parameters == b"?997;1" || parameters == b"?997;2",
            b't' => {
                (parameters.starts_with(b"4;") || parameters.starts_with(b"6;"))
                    && parameters
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || *byte == b';')
            }
            b'c' => {
                parameters.starts_with(b"?")
                    && parameters[1..]
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || *byte == b';')
            }
            _ => false,
        };
        return if known {
            Sequence::Complete
        } else {
            Sequence::Unrelated
        };
    }
    if body
        .iter()
        .all(|byte| byte.is_ascii_digit() || matches!(*byte, b'?' | b';'))
    {
        if body.starts_with(b"?997;") || body.starts_with(b"4;") || body.starts_with(b"6;") {
            Sequence::Reply
        } else {
            Sequence::Prefix
        }
    } else {
        Sequence::Unrelated
    }
}
