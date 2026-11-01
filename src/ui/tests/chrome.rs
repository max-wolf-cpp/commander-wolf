use super::*;

#[test]
fn overflowing_titles_keep_symmetric_border_segments_and_three_dots() {
    use super::super::chrome::draw_titled_block;
    use crate::theme::BorderStyle;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders};

    for border in ["plain", "rounded", "double", "thick", "simple"] {
        let border_set = BorderStyle::parse(border).unwrap().border_set();
        for width in [13, 14, 18, 30] {
            for title in [
                "abcdefghijklmnopqrstuvwxyz0123456789",
                "中文👩‍💻".repeat(10).as_str(),
                "e\u{301}".repeat(40).as_str(),
            ] {
                let mut terminal = Terminal::new(TestBackend::new(width, 3)).unwrap();
                terminal
                    .draw(|frame| {
                        draw_titled_block(
                            frame,
                            frame.area(),
                            Block::default()
                                .borders(Borders::ALL)
                                .border_set(border_set)
                                .border_style(Style::default().fg(Color::Red)),
                            title,
                            Style::default().fg(Color::Green),
                            Style::default(),
                        );
                    })
                    .unwrap();
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer[(0, 0)].symbol(), border_set.top_left);
                assert_eq!(buffer[(width - 1, 0)].symbol(), border_set.top_right);
                for x in [1, 2, 3, width - 4, width - 3, width - 2] {
                    assert_eq!(
                        buffer[(x, 0)].symbol(),
                        border_set.horizontal_top,
                        "{border} {width}"
                    );
                    assert_eq!(buffer[(x, 0)].fg, Color::Red);
                }
                for x in [4, width - 5] {
                    assert_eq!(buffer[(x, 0)].symbol(), " ");
                }
                for x in width - 8..width - 5 {
                    assert_eq!(buffer[(x, 0)].symbol(), ".");
                    assert_eq!(buffer[(x, 0)].fg, Color::Green);
                }
                for x in 5..width - 8 {
                    let symbol = buffer[(x, 0)].symbol();
                    assert_ne!(symbol, "\u{301}");
                    assert_ne!(symbol, "\u{200d}");
                }
            }
        }
    }
}

#[test]
fn fitting_titles_stay_complete_and_tiny_frames_keep_their_corners() {
    use super::super::chrome::draw_titled_block;
    use ratatui::style::Style;
    use ratatui::widgets::{Block, Borders};

    for width in 1..=24 {
        let mut terminal = Terminal::new(TestBackend::new(width, 3)).unwrap();
        terminal
            .draw(|frame| {
                draw_titled_block(
                    frame,
                    frame.area(),
                    Block::default().borders(Borders::ALL),
                    "exacttitle",
                    Style::default(),
                    Style::default(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        if width >= 2 {
            assert_eq!(buffer[(0, 0)].symbol(), "┌");
            assert_eq!(buffer[(width - 1, 0)].symbol(), "┐");
        }
        if width >= 20 {
            let title: String = (5..15).map(|x| buffer[(x, 0)].symbol()).collect();
            assert_eq!(title, "exacttitle");
            assert_eq!(buffer[(15, 0)].symbol(), " ");
        }
    }
}

#[test]
fn compact_frames_and_content_separators_use_independent_styles() {
    use crate::theme::{BorderStyle, SeparatorStyle};
    use ratatui::widgets::BorderType;
    for (panel_border, vertical, footer_border, footer_horizontal) in [
        (
            BorderStyle::Unicode(BorderType::Plain),
            "│",
            BorderStyle::Simple,
            "-",
        ),
        (
            BorderStyle::Unicode(BorderType::Rounded),
            "│",
            BorderStyle::Unicode(BorderType::Double),
            "═",
        ),
        (
            BorderStyle::Unicode(BorderType::Double),
            "║",
            BorderStyle::Unicode(BorderType::Thick),
            "━",
        ),
        (
            BorderStyle::Unicode(BorderType::Thick),
            "┃",
            BorderStyle::Unicode(BorderType::Plain),
            "─",
        ),
        (
            BorderStyle::Simple,
            "|",
            BorderStyle::Unicode(BorderType::Rounded),
            "─",
        ),
    ] {
        for (separator_style, horizontal) in [
            (SeparatorStyle::Thin, "─"),
            (SeparatorStyle::Rounded, "─"),
            (SeparatorStyle::Double, "═"),
            (SeparatorStyle::Thick, "━"),
            (SeparatorStyle::Simple, "-"),
            (SeparatorStyle::Dashed, "┄"),
            (SeparatorStyle::Off, " "),
        ] {
            let mut theme = Theme::emergency_monochrome();
            theme.panel_border = panel_border;
            theme.footer_border = footer_border;
            theme.separator_style = separator_style;
            let mut state = AppState::new(
                ".".into(),
                "test".into(),
                TerminalSize {
                    columns: 100,
                    rows: 50,
                },
                WorkspaceOptions::default(),
            );
            state.workspace.panel_style = PanelStyle::Compact;
            state.workspace.icon_mode = IconMode::None;
            state.workspace.preview_visible = true;
            state.preview.metadata = LoadState::Ready(crate::preview::FileMetadata {
                path: "test.txt".into(),
                file_type: "Regular file".into(),
                size: "0 bytes".into(),
                created: "unknown".into(),
                modified: "unknown".into(),
                permissions: "read/write".into(),
                extension: Some("txt".into()),
                hard_links: None,
            });
            let mut terminal = Terminal::new(TestBackend::new(100, 50)).unwrap();
            for footer in [UiStyle::Framed, UiStyle::Compact] {
                state.status_line_options.style = footer;
                let layout = resolve_layout_with_styles(
                    state.terminal_size.area(),
                    state.workspace.preview_panel(),
                    state.preview_options.media_percent,
                    footer,
                    PanelStyle::Compact,
                );
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                let buffer = terminal.backend().buffer();
                let separator = layout.bottom_separator;
                assert_eq!(
                    buffer[(separator.x + 1, separator.y)].symbol(),
                    if footer == UiStyle::Compact {
                        footer_horizontal
                    } else {
                        horizontal
                    }
                );
                assert_eq!(
                    buffer[(layout.panel_gap.x, layout.panel_gap.y + 1)].symbol(),
                    vertical
                );
                if footer == UiStyle::Framed {
                    assert_eq!(
                        buffer[(layout.bottom_frame.x, layout.bottom_frame.y)].symbol(),
                        footer_border.border_set().top_left
                    );
                    assert_eq!(buffer[(separator.x, separator.y)].symbol(), " ");
                    assert_eq!(buffer[(separator.right() - 1, separator.y)].symbol(), " ");
                    if separator_style != SeparatorStyle::Off {
                        assert_eq!(buffer[(separator.x + 1, separator.y)].fg, theme.line_number);
                    }
                } else {
                    for x in separator.x..separator.right() {
                        assert_eq!(buffer[(x, separator.y)].symbol(), footer_horizontal);
                    }
                }
                let preview = layout.preview.unwrap();
                assert_eq!(
                    buffer[(preview.separator.x + 1, preview.separator.y)].symbol(),
                    horizontal
                );
                assert_eq!(
                    buffer[(preview.metadata.x + 1, preview.metadata.y + 6)].symbol(),
                    horizontal
                );
            }
            state.finder.open(PanelId::Left, ".".into());
            terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
            let buffer = terminal.backend().buffer();
            let preview = finder::resolve_layout(state.terminal_size.area()).preview;
            for area in [preview.separator, preview.metadata] {
                let y = area.y + if area == preview.metadata { 6 } else { 0 };
                assert_eq!(buffer[(area.x, y)].symbol(), " ");
                assert_eq!(buffer[(area.right() - 1, y)].symbol(), " ");
                assert_eq!(buffer[(area.x + 1, y)].symbol(), horizontal);
            }
        }
    }
}

#[test]
fn separator_off_leaves_frames_and_clears_dividers() {
    let mut theme = Theme::emergency_monochrome();
    theme.set_border_style(crate::theme::BorderStyle::parse("double").unwrap());
    theme.separator_style = crate::theme::SeparatorStyle::Off;
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 50,
        },
        WorkspaceOptions::default(),
    );
    state.workspace.panel_style = PanelStyle::Compact;
    state.workspace.preview_visible = true;
    state.preview.metadata = LoadState::Ready(crate::preview::FileMetadata {
        path: "test.txt".into(),
        file_type: "Regular file".into(),
        size: "0 bytes".into(),
        created: "unknown".into(),
        modified: "unknown".into(),
        permissions: "read/write".into(),
        extension: Some("txt".into()),
        hard_links: None,
    });
    let layout = resolve_layout_with_styles(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
        UiStyle::Framed,
        PanelStyle::Compact,
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 50)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(layout.bottom_frame.x, layout.bottom_frame.y)].symbol(),
        "╔"
    );
    assert_eq!(
        buffer[(layout.bottom_separator.x + 1, layout.bottom_separator.y)].symbol(),
        " "
    );
    assert_eq!(
        buffer[(layout.panel_gap.x, layout.panel_gap.y + 1)].symbol(),
        "║"
    );
    let preview = layout.preview.unwrap();
    assert_eq!(
        buffer[(preview.separator.x + 1, preview.separator.y)].symbol(),
        " "
    );
    assert_eq!(
        buffer[(preview.metadata.x + 1, preview.metadata.y + 6)].symbol(),
        " "
    );
}

#[test]
fn style_can_remove_powerline_chrome_without_changing_frames() {
    let mut theme = Theme::emergency_monochrome();
    theme.decorations = false;
    theme.borderless_title_left = "".into();
    theme.borderless_title_right = "".into();
    theme.compact_mode_separator_left = "".into();
    theme.compact_mode_separator_right = "".into();
    for mode in [IconMode::Unicode, IconMode::None] {
        for style in [
            PanelStyle::Framed,
            PanelStyle::Borderless,
            PanelStyle::Compact,
        ] {
            for position in [ModePosition::Left, ModePosition::Right] {
                let mut state = AppState::new(
                    ".".into(),
                    "test".into(),
                    TerminalSize {
                        columns: 80,
                        rows: 24,
                    },
                    WorkspaceOptions {
                        icon_mode: mode,
                        panel_style: style,
                        ..WorkspaceOptions::default()
                    },
                );
                state.status_line_options.style = UiStyle::Compact;
                state.status_line_options.mode_position = position;
                let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                let text = buffer_text(terminal.backend().buffer());
                assert!(!text.contains(['', '', '', '']));
                assert!(text.contains("NORMAL"));
                if mode == IconMode::None && style == PanelStyle::Framed {
                    assert!(text.contains('┌'));
                }
            }
        }
    }
}

#[test]
fn simple_frame_and_dividers_are_explicit_even_with_icons() {
    let mut theme = Theme::emergency_monochrome();
    theme.set_border_style(crate::theme::BorderStyle::Simple);
    theme.separator_style = crate::theme::SeparatorStyle::Simple;
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 40,
            rows: 15,
        },
        WorkspaceOptions::default(),
    );
    state.workspace.icon_mode = IconMode::NerdFont;
    let mut terminal = Terminal::new(TestBackend::new(40, 15)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "+");
    assert_eq!(buffer[(1, 0)].symbol(), "-");
    assert_eq!(buffer[(0, 1)].symbol(), "|");
}
