use super::*;

#[test]
fn modal_viewer_draw_is_immutable_and_protects_its_media_rect() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state
        .viewer
        .open(PathBuf::from("画像.png"), "画像.png".into());
    state.viewer.layout = Some(ImageViewerLayout {
        modal: ratatui::layout::Rect::new(20, 5, 40, 14),
        media: ratatui::layout::Rect::new(22, 7, 36, 10),
        pixel_width: 360,
        pixel_height: 200,
    });
    state.viewer.status = ViewerStatus::Ready;
    let before = state.clone();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| draw(frame, &state, &Theme::emergency_monochrome()))
        .unwrap();
    assert_eq!(state, before);
    assert_eq!(
        terminal.backend().buffer()[(22, 7)].bg,
        ratatui::style::Color::Reset
    );
}

#[test]
fn modal_viewer_paints_its_media_background_before_presenting_the_image() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state
        .viewer
        .open(PathBuf::from("transparent.png"), "transparent.png".into());
    state.viewer.layout = Some(ImageViewerLayout {
        modal: ratatui::layout::Rect::new(20, 5, 40, 14),
        media: ratatui::layout::Rect::new(22, 7, 36, 10),
        pixel_width: 360,
        pixel_height: 200,
    });
    state.viewer.status = ViewerStatus::Preparing;
    let mut theme = Theme::emergency_monochrome();
    theme.float_background = Rgb {
        red: 12,
        green: 34,
        blue: 56,
    }
    .into();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();

    assert_eq!(
        terminal.backend().buffer()[(22, 7)].bg,
        theme.float_background
    );
}

#[test]
fn closing_the_viewer_redraws_its_previous_area_before_restoring_media() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.workspace.preview_visible = true;
    state.preview.title = "preview.png".into();
    state.preview.media = LoadState::Ready(MediaMetadata {
        kind: PreviewKind::Image,
        pixel_width: 20,
        pixel_height: 10,
        duration: None,
        has_audio: None,
    });
    state
        .viewer
        .open(PathBuf::from("modal-marker.png"), "modal-marker.png".into());
    state.viewer.layout = Some(ImageViewerLayout {
        modal: ratatui::layout::Rect::new(20, 5, 40, 14),
        media: ratatui::layout::Rect::new(22, 7, 36, 10),
        pixel_width: 360,
        pixel_height: 200,
    });
    state.viewer.status = ViewerStatus::Ready;
    let theme = Theme::emergency_monochrome();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("modal-marker.png"));

    state.viewer.close();
    state.preview.media = LoadState::Loading;
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert!(!buffer_text(buffer).contains("modal-marker.png"));
    assert_eq!(buffer[(42, 7)].bg, theme.preview_background);
}
