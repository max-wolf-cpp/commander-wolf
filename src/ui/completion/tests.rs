use super::*;
use crate::commands::completion::CompletionItem;
use crate::icons::IconMode;
use crate::terminal::TerminalSize;
use crate::workspace::WorkspaceOptions;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn title_does_not_recolor_the_rest_of_the_border() {
    use crate::theme::Rgb;
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.mode = AppMode::Command;
    state.command_completion = CommandCompletion::new(
        ":c".into(),
        vec![CompletionItem {
            name: "cd".into(),
            description: "Change the active panel directory".into(),
        }],
        false,
    );
    let mut theme = Theme::emergency_monochrome();
    theme.active_border = Rgb {
        red: 137,
        green: 180,
        blue: 250,
    }
    .into();
    theme.active_title = Rgb {
        red: 166,
        green: 227,
        blue: 161,
    }
    .into();
    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        state.workspace.icon_mode = mode;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let command_line = Rect::new(0, 23, 80, 1);
        let area = popup_area(
            Rect::new(0, 0, 80, 24),
            command_line,
            state.command_completion.as_ref().unwrap(),
        );
        terminal
            .draw(|frame| draw(frame, command_line, &state, &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        for x in area.x..area.right() {
            if !(area.x + 4..area.x + 14).contains(&x) {
                assert_eq!(
                    buffer[(x, area.y)].fg,
                    theme.active_border,
                    "border at column {x}"
                );
            }
            assert_eq!(buffer[(x, area.bottom() - 1)].fg, theme.active_border);
        }
        assert_eq!(buffer[(area.x + 5, area.y)].fg, theme.active_title);
    }
}

#[test]
fn argument_popup_shows_only_values_in_a_compact_frame() {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.mode = AppMode::Command;
    state.command_completion = CommandCompletion::new(
        ":lines ".into(),
        ["double", "thick", "thin"]
            .map(|name| CompletionItem {
                name: name.into(),
                description: String::new(),
            })
            .to_vec(),
        false,
    );
    let completion = state.command_completion.as_ref().unwrap();
    let command_line = Rect::new(0, 23, 80, 1);
    let area = popup_area(Rect::new(0, 0, 80, 24), command_line, completion);
    assert!(area.width < 20);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| draw(frame, command_line, &state, &Theme::emergency_monochrome()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let top: String = (area.x..area.right())
        .map(|x| buffer[(x, area.y)].symbol())
        .collect();
    assert!(!top.contains("Commands"));
    let first: String = (area.x..area.right())
        .map(|x| buffer[(x, area.y + 1)].symbol())
        .collect();
    assert!(first.contains("double"));
}

#[test]
fn popup_is_bounded_scrolls_and_keeps_the_input_cursor() {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.mode = AppMode::Command;
    state.command_completion = CommandCompletion::new(
        ":".into(),
        (0..60)
            .map(|i| CompletionItem {
                name: format!("command{i:02}"),
                description: format!("Description {i:02}"),
            })
            .collect(),
        true,
    );
    let theme = Theme::emergency_monochrome();
    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        state.workspace.icon_mode = mode;
        for (width, height) in [(80, 24), (30, 12), (10, 6), (4, 4), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let command_line = Rect::new(0, height - 1, width, 1);
            let area = popup_area(
                Rect::new(0, 0, width, height),
                command_line,
                state.command_completion.as_ref().unwrap(),
            );
            assert!(area.height <= height / 2);
            assert_eq!(area.bottom(), command_line.y);
            terminal
                .draw(|frame| {
                    frame.set_cursor_position((0, height - 1));
                    draw(frame, command_line, &state, &theme);
                })
                .unwrap();
            assert_eq!(terminal.backend().cursor_position(), (0, height - 1).into());
            if width == 80 {
                let buffer = terminal.backend().buffer();
                let last: String = (area.x..area.right())
                    .map(|x| buffer[(x, area.bottom() - 2)].symbol())
                    .collect();
                assert!(last.contains("command59"));
                assert!(last.contains("Description 59"));
                assert_eq!(
                    buffer[(area.right() - 1, area.bottom() - 2)].symbol(),
                    if mode == IconMode::None { "#" } else { "┃" }
                );
            }
        }
    }
}

#[test]
fn popup_follows_each_footer_without_covering_input_or_leaving_stale_cells() {
    use crate::status_line::UiStyle;
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 40,
        },
        WorkspaceOptions::default(),
    );
    state.mode = AppMode::Command;
    let completion = CommandCompletion::new(
        ":long".into(),
        vec![CompletionItem {
            name: "long_custom_command_狼".into(),
            description: "Open".into(),
        }],
        false,
    )
    .unwrap();
    let theme = Theme::emergency_monochrome();
    for style in [UiStyle::Framed, UiStyle::Borderless, UiStyle::Compact] {
        state.status_line_options.style = style;
        state.command_line = completion.line();
        state.command_completion = Some(completion.clone());
        let layout =
            crate::ui::resolve_layout_with_style(Rect::new(0, 0, 100, 40), None, 70, style);
        let area = popup_area(Rect::new(0, 0, 100, 40), layout.command_line, &completion);
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let row: String = (area.x..area.right())
            .map(|x| buffer[(x, area.y + 1)].symbol())
            .collect();
        assert!(row.contains("long_custom_command_狼"));
        assert!(row.contains("Open"));
        assert!(terminal.backend().cursor_position().y >= area.bottom());
        state.command_completion = None;
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let mut baseline = Terminal::new(TestBackend::new(100, 40)).unwrap();
        baseline
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        assert_eq!(terminal.backend().buffer(), baseline.backend().buffer());
    }
}
