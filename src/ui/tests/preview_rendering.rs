use super::*;

#[test]
fn long_metadata_paths_wrap_under_the_value_in_panels_and_finder() {
    use crate::preview::FileMetadata;
    use crate::workspace::PanelStyle;
    use ratatui::layout::Margin;

    let path = r"g:\Downloads\[DL] [In progress] Heroes of Might and Magic Olden Era [P] [RUS + ENG] 2026\game.zip";
    for columns in [60, 100, 180] {
        for panel_style in [
            PanelStyle::Framed,
            PanelStyle::Borderless,
            PanelStyle::Compact,
        ] {
            for finder_open in [false, true] {
                let mut state = AppState::new(
                    ".".into(),
                    "test".into(),
                    TerminalSize { columns, rows: 100 },
                    WorkspaceOptions {
                        panel_style,
                        ..WorkspaceOptions::default()
                    },
                );
                state.workspace.preview_visible = true;
                if finder_open {
                    state.finder.open(PanelId::Left, ".".into());
                }
                state.preview.metadata = LoadState::Ready(FileMetadata {
                    path: path.into(),
                    file_type: "Regular file".into(),
                    size: "1 B".into(),
                    created: "unknown".into(),
                    modified: "unknown".into(),
                    permissions: "Read/write".into(),
                    extension: Some("zip".into()),
                    hard_links: Some(1),
                });
                let preview = if finder_open {
                    finder_preview_layout(state.terminal_size.area())
                } else {
                    resolve_layout_with_styles(
                        state.terminal_size.area(),
                        state.workspace.preview_panel(),
                        state.preview_options.media_percent,
                        state.status_line_options.style,
                        panel_style,
                    )
                    .preview
                    .unwrap()
                };
                let metadata = preview.metadata.inner(Margin::new(1, 0));
                let value_x = metadata.x + 13;
                let value_width = usize::from(metadata.width - 13);
                let theme = Theme::emergency_monochrome();
                let mut terminal = Terminal::new(TestBackend::new(columns, 100)).unwrap();
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                let buffer = terminal.backend().buffer();
                for (offset, chunk) in path.as_bytes().chunks(value_width).enumerate() {
                    let y = metadata.y + offset as u16;
                    assert!(y < metadata.bottom());
                    let text: String = (0..chunk.len())
                        .map(|i| buffer[(value_x + i as u16, y)].symbol())
                        .collect();
                    assert_eq!(
                        text.as_bytes(),
                        chunk,
                        "{columns} {panel_style:?} finder={finder_open}"
                    );
                    if offset > 0 {
                        for x in metadata.x..value_x {
                            assert_eq!(buffer[(x, y)].symbol(), " ");
                        }
                    }
                }
                let type_y = metadata.y + path.len().div_ceil(value_width) as u16;
                let label: String = (0..4)
                    .map(|i| buffer[(metadata.x + i, type_y)].symbol())
                    .collect();
                assert_eq!(label, "Type");
                assert_eq!(buffer[(value_x, type_y)].symbol(), "R");
            }
        }
    }
}

#[test]
fn suspended_image_preview_erases_a_closed_progress_float() {
    use crate::file_operations::FileOperationKind;
    use crate::file_operations::progress::OperationProgress;
    use ratatui::buffer::CellDiffOption;
    use ratatui::style::Color;

    for background in [Color::Reset, Color::White, Color::Rgb(30, 30, 46)] {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 100,
                rows: 30,
            },
            WorkspaceOptions::default(),
        );
        state.workspace.preview_visible = true;
        state.preview.media = LoadState::Ready(MediaMetadata {
            kind: PreviewKind::Image,
            pixel_width: 800,
            pixel_height: 600,
            duration: None,
            has_audio: None,
        });
        state.preview.surface_suspended = true;
        state.operation_progress = Some(OperationProgress::new(FileOperationKind::Copy));
        let mut theme = Theme::emergency_monochrome();
        theme.preview_background = background;
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();

        state.operation_progress = None;
        let preview = resolve_layout_with_preview(
            state.terminal_size.area(),
            state.workspace.preview_panel(),
            state.preview_options.media_percent,
        )
        .preview
        .unwrap();
        let completed = terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
        for y in preview.media.y..preview.media.bottom() {
            for x in preview.media.x..preview.media.right() {
                let cell = &completed.buffer[(x, y)];
                assert_eq!(cell.symbol(), " ");
                assert_eq!(cell.bg, background);
                assert_ne!(cell.diff_option, CellDiffOption::Skip);
            }
        }
        for y in preview.media.y..preview.media.bottom() {
            for x in preview.media.x..preview.media.right() {
                assert_eq!(terminal.backend().buffer()[(x, y)].symbol(), " ");
            }
        }
    }
}

#[test]
fn rich_preview_protects_only_the_media_region() {
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
    state.preview.title = "image.png".into();
    state.preview.media = LoadState::Ready(MediaMetadata {
        kind: PreviewKind::Image,
        pixel_width: 20,
        pixel_height: 10,
        duration: None,
        has_audio: None,
    });
    state.preview.metadata = LoadState::Unavailable("metadata marker".into());
    let mut theme = Theme::emergency_monochrome();
    theme.preview_metadata_label = Rgb {
        red: 1,
        green: 2,
        blue: 3,
    }
    .into();
    theme.preview_metadata_value = Rgb {
        red: 4,
        green: 5,
        blue: 6,
    }
    .into();
    let layout = resolve_layout_with_preview(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
    );
    let preview = layout.preview.unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(preview.media.x, preview.media.y)].bg,
        ratatui::style::Color::Reset
    );
    assert_eq!(
        buffer[(preview.metadata.x, preview.metadata.y)].bg,
        theme.preview_background.into()
    );
    assert_eq!(
        buffer[(preview.metadata.x + 1, preview.metadata.y)].fg,
        theme.preview_metadata_label.into()
    );
    assert_eq!(
        buffer[(preview.metadata.x + 14, preview.metadata.y)].fg,
        theme.preview_metadata_value.into()
    );
    assert!(buffer.content().iter().any(|cell| cell.symbol() == "M"));
    for area in [preview.separator, preview.metadata] {
        for y in area.y..area.bottom() {
            assert_eq!(buffer[(area.x, y)].symbol(), " ");
            assert_eq!(buffer[(area.right() - 1, y)].symbol(), " ");
        }
    }
    assert_eq!(
        buffer[(preview.separator.x + 1, preview.separator.y)].symbol(),
        "─"
    );
}

#[test]
fn syntax_colors_and_padding_match_in_panel_and_finder_previews() {
    let text = r#"{"count": 42}"#.to_owned();
    let lines = vec![text.clone()];
    let highlighting = crate::syntax::default_highlighter()
        .highlight(std::path::Path::new("settings.json"), &lines);
    let theme = Theme::emergency_monochrome();
    let palette = crate::syntax::SyntaxTheme::from_fields(
        &[
            ("text".into(), "#CDD6F4".into()),
            ("property".into(), "#89DCEB".into()),
            ("number".into(), "#FAB387".into()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap();
    for finder_open in [false, true] {
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 100,
                rows: 30,
            },
            WorkspaceOptions::default(),
        );
        state.workspace.preview_visible = true;
        state.syntax_theme = palette.clone();
        state.preview.kind = PreviewKind::Text;
        state.preview.text = LoadState::Ready(TextPreview {
            lines: lines.clone(),
            highlighting: highlighting.clone(),
            truncated: false,
        });
        let area = if finder_open {
            state.finder.open(PanelId::Left, ".".into());
            finder::resolve_layout(state.terminal_size.area())
                .preview
                .media
        } else {
            resolve_layout_with_preview(
                state.terminal_size.area(),
                state.workspace.preview_panel(),
                state.preview_options.media_percent,
            )
            .preview
            .unwrap()
            .media
        };
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(area.x, area.y)].symbol(), " ");
        assert_eq!(buffer[(area.right() - 1, area.y)].symbol(), " ");
        assert_eq!(
            buffer[(area.x + 1 + text.find("count").unwrap() as u16, area.y)].fg,
            Rgb::parse("#89DCEB").unwrap().into()
        );
        assert_eq!(
            buffer[(area.x + 1 + text.find("42").unwrap() as u16, area.y)].fg,
            Rgb::parse("#FAB387").unwrap().into()
        );
    }
}

#[test]
fn syntax_off_renders_cached_tokens_as_plain_text() {
    use crate::syntax::{SyntaxCategory, SyntaxSpan};
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.workspace.preview_visible = true;
    state.preview.kind = PreviewKind::Text;
    state.preview.text = LoadState::Ready(TextPreview {
        lines: vec!["fn main".into()],
        highlighting: Ok(vec![vec![SyntaxSpan {
            range: 0..2,
            category: SyntaxCategory::Keyword,
        }]]),
        truncated: false,
    });
    state.syntax_theme = crate::syntax::SyntaxTheme::from_fields(
        &[
            ("text".into(), "#CDD6F4".into()),
            ("keyword".into(), "#CBA6F7".into()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap();
    state.syntax_enabled = false;
    let theme = Theme::emergency_monochrome();
    let layout = resolve_layout_with_preview(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
    );
    let area = layout.preview.unwrap().media;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let cell = &terminal.backend().buffer()[(area.x + 1, area.y)];
    assert_eq!(cell.symbol(), "f");
    assert_eq!(cell.fg, theme.text);
    assert!(!cell.modifier.contains(ratatui::style::Modifier::ITALIC));
}

#[test]
fn text_preview_is_drawn_by_ratatui_without_protecting_the_region() {
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
    state.preview.title = "main.rs".into();
    state.preview.kind = PreviewKind::Text;
    state.preview.media = LoadState::Idle;
    state.preview.text = LoadState::Ready(TextPreview {
        highlighting: Ok(Vec::new()),
        lines: vec![
            "fn main() {".into(),
            "    println!(\"Commander Wolf\");".into(),
            "x".repeat(200),
        ],
        truncated: false,
    });
    let theme = Theme::emergency_monochrome();
    let layout = resolve_layout_with_preview(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
    )
    .preview
    .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();

    let buffer = terminal.backend().buffer();
    assert!(buffer_text(buffer).contains("Commander Wolf"));
    for y in layout.media.y..layout.media.bottom() {
        assert_eq!(buffer[(layout.media.x, y)].symbol(), " ");
        assert_eq!(buffer[(layout.media.right() - 1, y)].symbol(), " ");
    }
    assert_eq!(buffer[(layout.media.x + 1, layout.media.y)].symbol(), "f");
    assert_eq!(
        layout.media.height,
        ((u32::from(layout.media.height + layout.metadata.height) * 70) / 100) as u16
    );
    assert_eq!(
        buffer[(layout.media.x, layout.media.y)].bg,
        theme.preview_background.into()
    );
}

#[test]
fn directory_preview_draws_icons_and_uses_an_explicit_empty_state() {
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
    state.workspace.icon_mode = IconMode::Unicode;
    state.preview.title = "folder".into();
    state.preview.kind = PreviewKind::Directory;
    state.preview.media = LoadState::Idle;
    state.preview.directory = LoadState::Ready(DirectoryPreview {
        entries: vec![
            DirectoryPreviewEntry {
                is_hidden: false,
                name: "Source".into(),
                is_directory: true,
            },
            DirectoryPreviewEntry {
                is_hidden: false,
                name: "README.md".into(),
                is_directory: false,
            },
        ],
        truncated: false,
    });
    let theme = Theme::emergency_monochrome();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let content = buffer_text(terminal.backend().buffer());
    assert!(content.contains("Source"));
    assert!(content.contains("README.md"));
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == "🗀")
    );
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.symbol() == "🗎")
    );

    state.preview.directory = LoadState::Ready(DirectoryPreview {
        entries: Vec::new(),
        truncated: false,
    });
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("Empty directory"));
}

#[test]
fn archive_preview_draws_internal_paths_and_an_explicit_empty_state() {
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
    state.preview.title = "stats.zip".into();
    state.preview.kind = PreviewKind::Archive;
    state.preview.media = LoadState::Idle;
    state.preview.archive = LoadState::Ready(ArchivePreview {
        entries: vec![ArchivePreviewEntry {
            path: "reports/summary.json".into(),
            is_directory: false,
        }],
        truncated: false,
    });
    let theme = Theme::emergency_monochrome();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("reports/summary.json"));

    state.preview.archive = LoadState::Ready(ArchivePreview {
        entries: Vec::new(),
        truncated: false,
    });
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("Empty archive"));
}

#[test]
fn loading_a_new_preview_clears_the_previous_fallback_before_media_is_written() {
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
    state.preview.title = "directory".into();
    state.preview.media = LoadState::Unavailable("old fallback marker".into());
    let theme = Theme::emergency_monochrome();
    let layout = resolve_layout_with_preview(
        state.terminal_size.area(),
        state.workspace.preview_panel(),
        state.preview_options.media_percent,
    )
    .preview
    .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("old fallback marker"));

    state.preview.title = "image.png".into();
    state.preview.media = LoadState::Loading;
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert!(!buffer_text(buffer).contains("old fallback marker"));
    assert_eq!(
        buffer[(layout.media.x, layout.media.y)].bg,
        theme.preview_background.into()
    );
}
