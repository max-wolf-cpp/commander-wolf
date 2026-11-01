use super::rows::{line_number, line_number_span, name_spans};
use super::*;
use crate::theme::Rgb;
use crate::workspace::DirectoryEntry;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::symbols;

#[test]
fn relative_numbers_keep_selected_absolute_and_left_aligned() {
    assert_eq!(line_number(LineNumberMode::Relative, 5, 5), "6    ");
    assert_eq!(line_number(LineNumberMode::Relative, 2, 5), "   3 ");
    assert_eq!(line_number(LineNumberMode::Absolute, 2, 5), "   3 ");
    assert_eq!(line_number(LineNumberMode::Off, 2, 5), "");

    let mut theme = Theme::emergency_monochrome();
    theme.line_number = Rgb {
        red: 1,
        green: 2,
        blue: 3,
    }
    .into();
    theme.current_line_number = Rgb {
        red: 4,
        green: 5,
        blue: 6,
    }
    .into();
    assert_eq!(
        line_number_span(LineNumberMode::Relative, 2, 5, &theme)
            .style
            .fg,
        Some(theme.line_number)
    );
    assert_eq!(
        line_number_span(LineNumberMode::Relative, 5, 5, &theme)
            .style
            .fg,
        Some(theme.current_line_number)
    );
}

#[test]
fn title_padding_does_not_own_the_leading_border_segment() {
    let mut terminal = Terminal::new(TestBackend::new(24, 3)).unwrap();
    terminal
        .draw(|frame| {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_set(symbols::border::DOUBLE)
                .border_style(Style::default().fg(Color::Red));
            draw_titled_block(
                frame,
                frame.area(),
                block,
                "title",
                Style::default().fg(Color::Green),
                Style::default(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "╔");
    for column in 1..=3 {
        assert_eq!(buffer[(column, 0)].symbol(), "═");
        assert_eq!(buffer[(column, 0)].fg, Color::Red);
    }
    assert_eq!(buffer[(4, 0)].symbol(), " ");
    assert_eq!(buffer[(5, 0)].symbol(), "t");
    assert_eq!(buffer[(5, 0)].fg, Color::Green);
}

#[test]
fn search_styles_every_occurrence_and_distinguishes_the_current_match() {
    let theme = Theme::emergency_monochrome();
    let matches = name_spans("foo-foo", Some("foo"), false, &theme, theme.text);
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].content.as_ref(), "foo");
    assert_eq!(matches[2].content.as_ref(), "foo");
    assert_eq!(matches[0].style.fg, Some(theme.search_match_text));
    let current = name_spans("foo", Some("foo"), true, &theme, theme.text);
    assert_eq!(current[0].style.fg, Some(theme.search_current_match_text));
    assert_eq!(
        current[0].style.bg,
        Some(theme.search_current_match_background)
    );
}

#[test]
fn marked_entries_render_a_stable_marker_column() {
    let mut panel = PanelState {
        entries: vec![crate::workspace::DirectoryEntry {
            path: "marked.txt".into(),
            name: "marked.txt".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }],
        ..PanelState::default()
    };
    panel.toggle_mark(std::path::Path::new("marked.txt"));
    let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();
    let search = SearchState::default();
    terminal
        .draw(|frame| {
            draw_panel(
                frame,
                frame.area(),
                &panel,
                LineNumberMode::Off,
                true,
                PanelOverlay {
                    search: (PanelId::Left, &search),
                    edit: None,
                    visual_anchor: None,
                    icon_mode: IconMode::NerdFont,
                    color_file_names: false,
                    panel_style: PanelStyle::Framed,
                },
                &Theme::emergency_monochrome(),
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert!(buffer.content().iter().any(|cell| cell.symbol() == "●"));
}

#[test]
fn nerd_font_file_icon_keeps_its_upstream_color() {
    let panel = PanelState {
        entries: vec![DirectoryEntry {
            path: "main.rs".into(),
            name: "main.rs".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }],
        ..PanelState::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();
    let search = SearchState::default();
    let theme = Theme::emergency_monochrome();
    terminal
        .draw(|frame| {
            draw_panel(
                frame,
                frame.area(),
                &panel,
                LineNumberMode::Off,
                true,
                PanelOverlay {
                    search: (PanelId::Left, &search),
                    edit: None,
                    visual_anchor: None,
                    icon_mode: IconMode::NerdFont,
                    color_file_names: false,
                    panel_style: PanelStyle::Framed,
                },
                &theme,
            );
        })
        .unwrap();
    let icon = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "")
        .expect("Rust devicon must be rendered");
    assert_eq!(icon.fg, crate::theme::Rgb::parse("#DEA584").unwrap().into());
}

#[test]
fn selected_text_color_overrides_icons_on_the_selected_row() {
    let panel = PanelState {
        entries: vec![DirectoryEntry {
            path: "main.rs".into(),
            name: "main.rs".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }],
        ..PanelState::default()
    };
    let mut theme = Theme::emergency_monochrome();
    theme.selected = Color::Indexed(7);
    theme.selected_text = Color::Indexed(0);
    let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();
    let mut search = SearchState::default();
    terminal
        .draw(|frame| {
            draw_panel(
                frame,
                frame.area(),
                &panel,
                LineNumberMode::Off,
                true,
                PanelOverlay {
                    search: (PanelId::Left, &search),
                    edit: None,
                    visual_anchor: None,
                    icon_mode: IconMode::NerdFont,
                    color_file_names: false,
                    panel_style: PanelStyle::Framed,
                },
                &theme,
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    for symbol in ["", "m"] {
        let cell = buffer
            .content()
            .iter()
            .find(|cell| cell.symbol() == symbol)
            .unwrap();
        assert_eq!(cell.fg, Color::Indexed(0));
        assert_eq!(cell.bg, Color::Indexed(7));
    }

    search.highlight_panel = Some(PanelId::Left);
    search.highlight_query = Some("main".into());
    search.current_match = Some(0);
    terminal
        .draw(|frame| {
            draw_panel(
                frame,
                frame.area(),
                &panel,
                LineNumberMode::Off,
                true,
                PanelOverlay {
                    search: (PanelId::Left, &search),
                    edit: None,
                    visual_anchor: None,
                    icon_mode: IconMode::NerdFont,
                    color_file_names: false,
                    panel_style: PanelStyle::Framed,
                },
                &theme,
            );
        })
        .unwrap();
    let match_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "m")
        .unwrap();
    assert_eq!(match_cell.fg, theme.search_current_match_text);
    assert_eq!(match_cell.bg, theme.search_current_match_background);
}

#[test]
fn visual_anchor_highlights_the_contiguous_range() {
    let mut panel = PanelState {
        entries: (0..3)
            .map(|index| crate::workspace::DirectoryEntry {
                path: index.to_string().into(),
                name: index.to_string(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            })
            .collect(),
        selected: 2,
        ..PanelState::default()
    };
    panel.viewport.visible_rows = 3;
    let mut theme = Theme::emergency_monochrome();
    theme.selected = crate::theme::Rgb {
        red: 1,
        green: 2,
        blue: 3,
    }
    .into();
    theme.visual_selection_background = crate::theme::Rgb::parse("#F9E2AF").unwrap().into();
    theme.visual_selection_text = crate::theme::Rgb::parse("#1E1E2E").unwrap().into();
    let search = SearchState::default();
    let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
    terminal
        .draw(|frame| {
            draw_panel(
                frame,
                frame.area(),
                &panel,
                LineNumberMode::Off,
                true,
                PanelOverlay {
                    search: (PanelId::Left, &search),
                    edit: None,
                    visual_anchor: Some(0),
                    icon_mode: IconMode::NerdFont,
                    color_file_names: false,
                    panel_style: PanelStyle::Framed,
                },
                &theme,
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    for row in 1..=3 {
        for column in 1..29 {
            assert_eq!(buffer[(column, row)].bg, theme.visual_selection_background);
            assert_eq!(buffer[(column, row)].fg, theme.visual_selection_text);
        }
    }
}
