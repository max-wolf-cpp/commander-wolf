use super::*;

#[test]
fn status_and_command_lines_render_independently_with_padding_and_pending_keys() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "Kitty · 8x21 px".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.pending_keys = "gg".into();
    let mut theme = Theme::emergency_monochrome();
    theme.status_background = Rgb {
        red: 1,
        green: 2,
        blue: 3,
    }
    .into();
    theme.status_border = Rgb {
        red: 4,
        green: 5,
        blue: 6,
    }
    .into();
    theme.line_number = Rgb {
        red: 7,
        green: 8,
        blue: 9,
    }
    .into();
    theme.mode_normal = Rgb {
        red: 13,
        green: 14,
        blue: 15,
    }
    .into();
    theme.mode_command = Rgb {
        red: 16,
        green: 17,
        blue: 18,
    }
    .into();
    let layout = resolve_layout(state.terminal_size.area(), None);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(layout.bottom_frame.x, layout.bottom_frame.y)].fg,
        theme.mode_normal
    );
    assert_eq!(
        buffer[(layout.status_line.x, layout.status_line.y)].symbol(),
        "N"
    );
    assert_eq!(
        buffer[(layout.status_line.x, layout.status_line.y)].fg,
        theme.mode_normal
    );
    assert_eq!(
        buffer[(layout.command_line.right() - 2, layout.command_line.y)].symbol(),
        "g"
    );
    assert_eq!(
        buffer[(layout.command_line.right() - 1, layout.command_line.y)].symbol(),
        "g"
    );
    assert_eq!(layout.bottom_frame.bottom(), 24);

    state.mode = crate::app::AppMode::Command;
    state.command_line = ":q".into();
    state.pending_keys.clear();
    state.status_line_options.mode_position = ModePosition::Right;
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(layout.command_line.x, layout.command_line.y)].symbol(),
        ":"
    );
    let mode_x = layout.status_line.right() - "COMMAND".len() as u16;
    assert_eq!(buffer[(mode_x, layout.status_line.y)].symbol(), "C");
    assert_eq!(
        buffer[(mode_x, layout.status_line.y)].fg,
        theme.mode_command
    );
    assert_eq!(
        buffer[(layout.bottom_frame.x, layout.bottom_frame.y)].fg,
        theme.mode_command
    );

    theme.framed_footer_border_follows_mode = false;
    theme.framed_footer_border = Rgb {
        red: 22,
        green: 23,
        blue: 24,
    }
    .into();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(layout.bottom_frame.x, layout.bottom_frame.y)].fg,
        theme.framed_footer_border
    );

    state.status_line_options.mode_position = ModePosition::Left;
    state.status_line_options.style = UiStyle::Borderless;
    state.pending_keys = "gg".into();
    theme.compact_status_background = Rgb {
        red: 19,
        green: 20,
        blue: 21,
    }
    .into();
    theme.compact_mode_separator_left = "".into();
    let compact = resolve_layout_with_style(
        state.terminal_size.area(),
        None,
        state.preview_options.media_percent,
        UiStyle::Borderless,
    );
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(compact.status_line.x, compact.status_line.y)].symbol(),
        " "
    );
    assert_eq!(
        buffer[(compact.status_line.x, compact.status_line.y)].bg,
        theme.mode_command
    );
    assert_eq!(
        buffer[(compact.status_line.x + 1, compact.status_line.y)].symbol(),
        "C"
    );
    assert_eq!(
        buffer[(compact.status_line.x + 9, compact.status_line.y)].symbol(),
        ""
    );
    assert_eq!(
        buffer[(compact.status_line.right() - 1, compact.status_line.y)].bg,
        theme.compact_status_background
    );
    assert_eq!(
        buffer[(compact.command_line.x, compact.command_line.y)].symbol(),
        ":"
    );
    assert_eq!(
        buffer[(compact.command_line.right() - 2, compact.command_line.y)].symbol(),
        "g"
    );
    assert_eq!(
        buffer[(compact.command_line.right() - 1, compact.command_line.y)].symbol(),
        "g"
    );
    assert!(compact.bottom_spacer.is_empty());

    state.status_line_options.style = UiStyle::Framed;
    let divided = resolve_layout_with_style(
        state.terminal_size.area(),
        None,
        state.preview_options.media_percent,
        UiStyle::Framed,
    );
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(divided.status_line.x, divided.status_line.y)].symbol(),
        "C"
    );
    assert_eq!(
        buffer[(divided.status_line.x, divided.status_line.y)].fg,
        theme.mode_command
    );
    assert_eq!(
        buffer[(divided.status_line.x, divided.status_line.y)].bg,
        theme.status_background
    );
    assert_eq!(
        buffer[(divided.bottom_separator.x, divided.bottom_separator.y)].symbol(),
        " "
    );
    assert_eq!(
        buffer[(divided.bottom_separator.x + 1, divided.bottom_separator.y)].symbol(),
        "─"
    );
    assert_eq!(
        buffer[(divided.bottom_separator.x + 1, divided.bottom_separator.y)].fg,
        theme.line_number
    );
    assert_eq!(
        buffer[(divided.command_line.x, divided.command_line.y)].symbol(),
        ":"
    );
    assert_eq!(divided.bottom_frame.bottom(), 24);

    state.status_line_options.style = UiStyle::Compact;
    let separated = resolve_layout_with_style(
        state.terminal_size.area(),
        None,
        state.preview_options.media_percent,
        UiStyle::Compact,
    );
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(separated.bottom_separator.x, separated.bottom_separator.y)].symbol(),
        "─"
    );
    assert_eq!(
        buffer[(
            separated.bottom_separator.right() - 1,
            separated.bottom_separator.y
        )]
            .symbol(),
        "─"
    );
    assert_eq!(
        buffer[(separated.status_line.x, separated.status_line.y)].bg,
        theme.mode_command
    );
    assert_eq!(
        buffer[(separated.command_line.x, separated.command_line.y)].symbol(),
        ":"
    );
    assert_eq!(separated.command_line.bottom(), 24);
}
