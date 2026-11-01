use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::*;
use crate::input::{InputContext, KeyChord, KeyResolver, Keymaps, ResolveOutcome};

#[derive(Clone, Copy)]
enum Encoding {
    Windows,
    Unix,
}

#[test]
fn delayed_palette_replies_do_not_become_normal_mode_commands() {
    for encoding in [Encoding::Windows, Encoding::Unix] {
        let mut input = TerminalInput::default();
        let started = Instant::now();
        input.begin_appearance_query();
        input.push(key(KeyCode::Char('j')), started);
        let delayed = started + Duration::from_secs(2);
        feed(
            &mut input,
            b"\x1b]4;8;rgb:5858/5b5b/7070\x1b\\",
            encoding,
            delayed,
        );
        input.push(key(KeyCode::Tab), delayed);
        assert_eq!(
            input.ready,
            VecDeque::from([key(KeyCode::Char('j')), key(KeyCode::Tab)])
        );

        let mut maps = Keymaps::default();
        maps.set(InputContext::Normal, ":", "command.enter", " ")
            .unwrap();
        let mut resolver = KeyResolver::default();
        for event in input.ready {
            let Event::Key(key) = event else {
                unreachable!()
            };
            assert!(!matches!(
                resolver.resolve(&maps, InputContext::Normal, KeyChord::from(key)),
                ResolveOutcome::Command(invocation) if invocation.id.0 == "command.enter"
            ));
        }
    }
}

#[test]
fn split_background_replies_accept_bel_and_st_and_keep_keys_between_replies() {
    for encoding in [Encoding::Windows, Encoding::Unix] {
        for terminator in [b"\x07".as_slice(), b"\x1b\\".as_slice()] {
            let mut input = TerminalInput::default();
            let now = Instant::now();
            feed(&mut input, b"\x1b]11;rgb:ffff/", encoding, now);
            assert!(input.ready.is_empty());
            assert!(input.take_appearance().is_none());
            feed(
                &mut input,
                b"0000/8080",
                encoding,
                now + Duration::from_millis(200),
            );
            feed(
                &mut input,
                terminator,
                encoding,
                now + Duration::from_millis(201),
            );
            assert!(input.ready.is_empty());
            assert_eq!(
                input.take_appearance().unwrap().background,
                Some(Rgb {
                    red: 255,
                    green: 0,
                    blue: 128
                })
            );
            input.push(key(KeyCode::Down), now + Duration::from_millis(202));
            feed(
                &mut input,
                b"\x1b]4;1;rgb:ff/00/00\x07",
                encoding,
                now + Duration::from_millis(203),
            );
            input.push(key(KeyCode::Char('k')), now + Duration::from_millis(204));
            assert_eq!(
                input.ready,
                VecDeque::from([key(KeyCode::Down), key(KeyCode::Char('k'))])
            );
        }
    }
}

#[test]
fn queued_reply_fragments_survive_slow_frame_processing() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    feed(&mut input, b"\x1b]", Encoding::Windows, now);
    feed(
        &mut input,
        b"11;rgb:ff/00/80\x07",
        Encoding::Windows,
        now + Duration::from_secs(2),
    );
    assert!(input.ready.is_empty());
    assert_eq!(
        input.take_appearance().unwrap().background,
        Some(Rgb {
            red: 255,
            green: 0,
            blue: 128
        })
    );
}

#[test]
fn ordinary_text_unicode_modifiers_and_event_kinds_are_preserved() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    let mut events = ":rgb:ffff/0000/8080 Привет"
        .chars()
        .map(|character| key(KeyCode::Char(character)))
        .collect::<Vec<_>>();
    events.extend([
        key(KeyCode::Up),
        Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
        Event::Paste("rgb:ff/00/80".into()),
        Event::Resize(100, 30),
        Event::FocusGained,
    ]);
    for event in &events {
        input.push(event.clone(), now);
    }
    assert_eq!(input.ready, VecDeque::from(events));
    assert!(input.take_appearance().is_none());
}

#[test]
fn escape_and_unrelated_alt_sequences_are_replayed_without_losing_modifiers() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    let escape = key(KeyCode::Esc);
    input.push(escape.clone(), now);
    input
        .replies
        .expire(now + Duration::from_millis(50), &mut input.ready);
    assert_eq!(input.ready.pop_front(), Some(escape));

    let alt = Event::Key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::ALT));
    let next = key(KeyCode::Char('x'));
    input.push(alt.clone(), now + Duration::from_millis(60));
    input.push(next.clone(), now + Duration::from_millis(61));
    assert_eq!(input.ready, VecDeque::from([alt, next]));
}

#[test]
fn incomplete_replies_expire_and_oversized_replies_stop_at_the_terminator() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    feed(&mut input, b"\x1b]11;rgb:ff/", Encoding::Windows, now);
    let after_timeout = now + Duration::from_secs(1);
    input.replies.expire(after_timeout, &mut input.ready);
    input.push(key(KeyCode::Up), after_timeout);
    assert_eq!(input.ready.pop_front(), Some(key(KeyCode::Up)));

    feed(
        &mut input,
        b"\x1b]11;rgb:",
        Encoding::Windows,
        after_timeout,
    );
    feed(
        &mut input,
        &vec![b'f'; 4096],
        Encoding::Windows,
        after_timeout,
    );
    feed(&mut input, b"\x1b\\", Encoding::Windows, after_timeout);
    input.push(key(KeyCode::Tab), after_timeout);
    assert_eq!(input.ready, VecDeque::from([key(KeyCode::Tab)]));
    assert!(input.take_appearance().is_none());
}

#[test]
fn a_new_reply_recovers_from_a_truncated_control_string() {
    for encoding in [Encoding::Windows, Encoding::Unix] {
        let mut input = TerminalInput::default();
        let now = Instant::now();
        feed(&mut input, b"\x1b]11;rgb:bad", encoding, now);
        feed(&mut input, b"\x1b]11;rgb:00/ff/00\x07", encoding, now);
        assert!(input.ready.is_empty());
        assert_eq!(
            input.take_appearance().unwrap().background,
            Some(Rgb {
                red: 0,
                green: 255,
                blue: 0
            })
        );
    }
}

#[test]
fn startup_graphics_and_geometry_replies_do_not_reach_the_keymaps() {
    let now = Instant::now();
    for encoding in [Encoding::Windows, Encoding::Unix] {
        let mut input = TerminalInput::default();
        for reply in [
            b"\x1b_Gi=31;OK\x1b\\".as_slice(),
            b"\x1bP>|WezTerm 2026\x1b\\".as_slice(),
            b"\x1b]1337;Capabilities=RGB,Image\x07".as_slice(),
        ] {
            feed(&mut input, reply, encoding, now);
        }
        assert!(input.ready.is_empty());
        assert!(input.take_appearance().is_none());
    }
    // On Unix, Crossterm consumes CSI replies before returning key events.
    // Windows exposes their characters, so the filter also recognizes them.
    let mut input = TerminalInput::default();
    for reply in [
        b"\x1b[6;30;12t".as_slice(),
        b"\x1b[4;900;1600t".as_slice(),
        b"\x1b[?1;2;4c".as_slice(),
    ] {
        feed(&mut input, reply, Encoding::Windows, now);
    }
    assert!(input.ready.is_empty());
}

#[test]
fn complete_palettes_replace_the_previous_palette_only_after_all_colors_arrive() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    for generation in [0, 16] {
        input.begin_appearance_query();
        for index in 0..16 {
            let color = generation + index;
            let reply = format!("\x1b]4;{index};rgb:{color:02x}/00/00\x07");
            feed(&mut input, reply.as_bytes(), Encoding::Windows, now);
            if index < 15 {
                assert!(input.take_appearance().is_none());
            }
        }
        let palette = input.take_appearance().unwrap().palette.unwrap();
        assert_eq!(palette[15].red, generation + 15);
    }
}

#[test]
fn repeated_navigation_keys_between_palette_replies_are_not_lost() {
    for encoding in [Encoding::Windows, Encoding::Unix] {
        let mut input = TerminalInput::default();
        let now = Instant::now();
        let mut expected = VecDeque::new();
        for index in 0..16 {
            let key = key(if index % 2 == 0 {
                KeyCode::Char('j')
            } else {
                KeyCode::Down
            });
            input.push(key.clone(), now);
            expected.push_back(key);
            let reply = format!("\x1b]4;{index};rgb:58/5b/70\x1b\\");
            feed(&mut input, reply.as_bytes(), encoding, now);
        }
        assert_eq!(input.ready, expected);
    }
}

#[test]
fn malformed_color_reports_do_not_inject_text_or_block_the_following_key() {
    for encoding in [Encoding::Windows, Encoding::Unix] {
        let mut input = TerminalInput::default();
        let now = Instant::now();
        feed(&mut input, b"\x1b]11;rgb:bad:data:extra\x07", encoding, now);
        input.push(key(KeyCode::Char('j')), now);
        assert_eq!(input.ready, VecDeque::from([key(KeyCode::Char('j'))]));
        assert!(input.take_appearance().is_none());
    }
}

#[test]
fn windows_reply_key_releases_do_not_expose_reply_characters_as_commands() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    for &byte in b"\x1b]11;rgb:00/00/00\x1b\\" {
        let code = if byte == b'\x1b' {
            KeyCode::Esc
        } else {
            KeyCode::Char(char::from(byte))
        };
        input.push(key(code), now);
        input.push(
            Event::Key(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )),
            now,
        );
    }
    assert!(
        input
            .ready
            .iter()
            .all(|event| matches!(event, Event::Key(key) if key.kind == KeyEventKind::Release))
    );
    assert!(input.take_appearance().unwrap().background.is_some());
}

#[test]
fn palette_replies_between_leader_keys_do_not_break_the_mapping() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "<Leader>p", "preview.toggle", " ")
        .unwrap();
    let mut input = TerminalInput::default();
    let now = Instant::now();
    input.push(key(KeyCode::Char(' ')), now);
    feed(
        &mut input,
        b"\x1b]4;8;rgb:58/5b/70\x07",
        Encoding::Windows,
        now,
    );
    input.push(key(KeyCode::Char('p')), now);
    let mut resolver = KeyResolver::default();
    let outcomes = input
        .ready
        .into_iter()
        .map(|event| {
            let Event::Key(key) = event else {
                unreachable!()
            };
            resolver.resolve(&maps, InputContext::Normal, KeyChord::from(key))
        })
        .collect::<Vec<_>>();
    assert!(
        matches!(outcomes.as_slice(), [ResolveOutcome::Pending(_), ResolveOutcome::Command(invocation)] if invocation.id.0 == "preview.toggle")
    );
}

#[test]
fn focus_and_resize_notifications_do_not_release_partial_reply_introducers() {
    let mut input = TerminalInput::default();
    let now = Instant::now();
    feed(&mut input, b"\x1b", Encoding::Windows, now);
    input.push(Event::FocusGained, now);
    feed(&mut input, b"]", Encoding::Windows, now);
    input.push(Event::Resize(100, 30), now);
    feed(&mut input, b"11;rgb:00/00/00\x1b\\", Encoding::Windows, now);
    assert_eq!(
        input.ready,
        VecDeque::from([Event::FocusGained, Event::Resize(100, 30)])
    );
    assert!(input.take_appearance().unwrap().background.is_some());
}

#[test]
fn live_queries_use_only_osc_colors_and_leave_scheme_inference_to_the_background() {
    let request = crate::terminal_appearance::color_query();
    assert!(request.starts_with(b"\x1b]11;?\x07"));
    assert!(!request.windows(2).any(|window| window == b"\x1b["));
    for index in 0..16 {
        let expected = format!("\x1b]4;{index};?\x07");
        assert!(
            request
                .windows(expected.len())
                .any(|window| window == expected.as_bytes())
        );
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn feed(input: &mut TerminalInput, bytes: &[u8], encoding: Encoding, now: Instant) {
    let mut index = 0;
    while index < bytes.len() {
        if matches!(encoding, Encoding::Unix)
            && bytes[index] == b'\x1b'
            && bytes
                .get(index + 1)
                .is_some_and(|byte| matches!(*byte, b']' | b'_' | b'P' | b'\\'))
        {
            input.push(
                Event::Key(KeyEvent::new(
                    KeyCode::Char(char::from(bytes[index + 1])),
                    KeyModifiers::ALT,
                )),
                now,
            );
            index += 2;
        } else {
            let event = match bytes[index] {
                b'\x1b' => key(KeyCode::Esc),
                b'\x07' if matches!(encoding, Encoding::Unix) => {
                    Event::Key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL))
                }
                byte => key(KeyCode::Char(char::from(byte))),
            };
            input.push(event, now);
            index += 1;
        }
    }
}
