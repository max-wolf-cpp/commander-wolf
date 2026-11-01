use super::*;
use windows_sys::Win32::System::Console::{INPUT_RECORD_0, KEY_EVENT_RECORD, KEY_EVENT_RECORD_0};

use crate::runtime::terminal_input::TerminalInput;
use crate::theme::Rgb;

#[test]
fn captured_vk_zero_color_replies_retain_their_escape_delimiters() {
    // Captured from the failing Windows terminal: all 51 response records had
    // VK=0, scan=0, no modifiers, and key-down=true. The original event-only
    // tests assumed Crossterm had already preserved these ESC characters.
    let captured = b"\x1b]11;rgb:1e1e/1e1e/2e2e\x1b\\\x1b]4;8;rgb:5858/5b5b/7070\x1b\\";
    let mut input = TerminalInput::default();
    let now = std::time::Instant::now();
    for &byte in captured {
        let record = key_record(0, u16::from(byte), 0, true);
        let event = control_event(&record).unwrap_or_else(|| {
            // Printable VK=0 records use Crossterm's direct Unicode path.
            assert!(byte >= 0x20);
            Event::Key(KeyEvent::new(
                KeyCode::Char(char::from(byte)),
                KeyModifiers::NONE,
            ))
        });
        input.push(event, now);
    }
    assert!(
        input.ready.is_empty(),
        "terminal reply reached the keymap queue"
    );
    assert_eq!(
        input.take_appearance().unwrap().background,
        Some(Rgb {
            red: 30,
            green: 30,
            blue: 46
        })
    );
    assert_eq!(
        input.palette[8],
        Some(Rgb {
            red: 88,
            green: 91,
            blue: 112
        })
    );
}

#[test]
fn physical_keys_and_printable_vk_zero_records_remain_with_crossterm() {
    for record in [
        key_record(0x1b, 0x1b, 0, true),                 // Physical Escape.
        key_record(0x47, 0x07, LEFT_CTRL_PRESSED, true), // Physical Ctrl+G.
        key_record(0x28, 0, 0, true),                    // Down arrow, no Unicode character.
        key_record(0x4a, u16::from(b'j'), 0, true),
        key_record(0, u16::from(b':'), 0, true),
        key_record(0, 0xd83d, 0, true), // UTF-16 high surrogate.
        key_record(0, 0xde00, 0, true), // UTF-16 low surrogate.
    ] {
        assert!(control_event(&record).is_none());
    }
}

#[test]
fn synthetic_bel_and_escape_preserve_key_kind_and_modifiers() {
    let escape = key_record(
        0,
        0x1b,
        LEFT_ALT_PRESSED | LEFT_CTRL_PRESSED | SHIFT_PRESSED,
        false,
    );
    assert_eq!(
        control_event(&escape),
        Some(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Esc,
            KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyEventKind::Release,
        )))
    );
    let bell = key_record(0, 0x07, 0, true);
    assert_eq!(
        control_event(&bell),
        Some(Event::Key(KeyEvent::new(
            KeyCode::Char('\x07'),
            KeyModifiers::NONE
        )))
    );
}

#[test]
fn synthetic_bel_terminates_an_osc_reply_before_the_next_real_key() {
    let mut input = TerminalInput::default();
    let now = std::time::Instant::now();
    for &byte in b"\x1b]11;rgb:00/00/00\x07" {
        let record = key_record(0, u16::from(byte), 0, true);
        let event = control_event(&record).unwrap_or_else(|| {
            Event::Key(KeyEvent::new(
                KeyCode::Char(char::from(byte)),
                KeyModifiers::NONE,
            ))
        });
        input.push(event, now);
    }
    let next = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    input.push(next.clone(), now);
    assert_eq!(input.ready.pop_front(), Some(next));
    assert!(input.ready.is_empty());
    assert!(input.take_appearance().is_some());
}

fn key_record(virtual_key: u16, unicode: u16, modifiers: u32, down: bool) -> INPUT_RECORD {
    INPUT_RECORD {
        EventType: KEY_EVENT as u16,
        Event: INPUT_RECORD_0 {
            KeyEvent: KEY_EVENT_RECORD {
                bKeyDown: i32::from(down),
                wRepeatCount: 1,
                wVirtualKeyCode: virtual_key,
                wVirtualScanCode: 0,
                uChar: KEY_EVENT_RECORD_0 {
                    UnicodeChar: unicode,
                },
                dwControlKeyState: modifiers,
            },
        },
    }
}
