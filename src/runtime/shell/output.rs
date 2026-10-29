use std::io::{self, Read};
use std::sync::mpsc::SyncSender;
use std::thread;

pub(super) enum OutputEvent {
    Data(usize, Vec<u8>),
    Closed(usize),
    Error(String),
}

#[derive(Default)]
pub(super) struct Decoder {
    pending: Vec<u8>,
    escape: u8,
}

impl Decoder {
    pub(super) fn push(&mut self, bytes: &[u8], finished: bool) -> String {
        self.strip_escape_sequences(bytes);
        self.decode_pending_utf8(finished)
    }

    fn strip_escape_sequences(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            match self.escape {
                1 => {
                    self.escape = match byte {
                        b'[' => 2,
                        b']' | b'P' | b'_' | b'^' | b'X' => 3,
                        _ => 0,
                    }
                }
                2 => {
                    if (0x40..=0x7e).contains(&byte) {
                        self.escape = 0;
                    }
                }
                3 => {
                    if byte == 7 {
                        self.escape = 0;
                    } else if byte == 27 {
                        self.escape = 4;
                    }
                }
                4 => self.escape = if byte == b'\\' { 0 } else { 3 },
                _ => {
                    if byte == 27 {
                        self.escape = 1;
                    } else {
                        self.pending.push(byte);
                    }
                }
            }
        }
    }

    fn decode_pending_utf8(&mut self, finished: bool) -> String {
        let mut text = String::new();
        let mut count = 0;
        while count < self.pending.len() {
            match std::str::from_utf8(&self.pending[count..]) {
                Ok(rest) => {
                    text.push_str(rest);
                    count = self.pending.len();
                }
                Err(error) => {
                    let valid_end = count + error.valid_up_to();
                    text.push_str(
                        std::str::from_utf8(&self.pending[count..valid_end])
                            .expect("validated UTF-8 prefix"),
                    );
                    count = valid_end;
                    if let Some(length) = error.error_len() {
                        text.push('\u{fffd}');
                        count += length;
                    } else if finished {
                        text.push('\u{fffd}');
                        count = self.pending.len();
                    } else {
                        break;
                    }
                }
            }
        }

        self.pending.drain(..count);

        text
    }
}

pub(super) fn read_output(
    mut pipe: impl Read + Send + 'static,
    sender: SyncSender<OutputEvent>,
    stream: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    if sender
                        .send(OutputEvent::Data(stream, buffer[..count].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let _ = sender.send(OutputEvent::Error(error.to_string()));
                    break;
                }
            }
        }

        let _ = sender.send(OutputEvent::Closed(stream));
    })
}
