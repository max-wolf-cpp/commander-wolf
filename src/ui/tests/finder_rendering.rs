use super::*;

#[test]
fn finder_modal_is_immutable_and_safe_in_a_small_terminal() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 20,
            rows: 5,
        },
        WorkspaceOptions::default(),
    );
    state.finder.open(PanelId::Left, PathBuf::from("目录"));
    state.finder.pattern = "*.rs".into();
    let before = state.clone();
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    terminal
        .draw(|frame| draw(frame, &state, &Theme::emergency_monochrome()))
        .unwrap();
    assert_eq!(state, before);
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == ">")
    );
}

#[test]
fn finder_draws_results_input_and_existing_preview_with_static_titles() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        WorkspaceOptions::default(),
    );
    state
        .finder
        .open(PanelId::Left, PathBuf::from("root-must-not-be-a-title"));
    state.preview.title = "selected-name-must-not-be-a-title.rs".into();
    state.preview.kind = PreviewKind::Text;
    state.preview.text = LoadState::Ready(TextPreview {
        highlighting: Ok(Vec::new()),
        lines: vec!["finder preview marker".into(), "x".repeat(200)],
        truncated: false,
    });
    let mut theme = Theme::emergency_monochrome();
    theme.float_background = Rgb {
        red: 1,
        green: 2,
        blue: 3,
    }
    .into();
    theme.preview_background = Rgb {
        red: 4,
        green: 5,
        blue: 6,
    }
    .into();
    let finder_layout = finder::resolve_layout(state.terminal_size.area());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();

    let buffer = terminal.backend().buffer();
    let content = buffer_text(buffer);
    assert!(content.contains("Results"));
    assert!(content.contains("Find Files"));
    assert!(content.contains("Preview"));
    assert!(content.contains("finder preview marker"));
    assert!(!content.contains("root-must-not-be-a-title"));
    assert!(!content.contains("selected-name-must-not-be-a-title.rs"));
    for area in [
        finder_layout.preview.media,
        finder_layout.preview.separator,
        finder_layout.preview.metadata,
    ] {
        for y in area.y..area.bottom() {
            assert_eq!(buffer[(area.x, y)].symbol(), " ");
            assert_eq!(buffer[(area.right() - 1, y)].symbol(), " ");
        }
    }
    assert_eq!(
        buffer[(
            finder_layout.preview.separator.x + 1,
            finder_layout.preview.separator.y
        )]
            .symbol(),
        "─"
    );
    assert_eq!(
        buffer[(
            finder_layout.preview.metadata.x,
            finder_layout.preview.metadata.y
        )]
            .bg,
        theme.float_background.into()
    );
}
