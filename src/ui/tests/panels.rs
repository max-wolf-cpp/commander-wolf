use super::*;

#[test]
fn file_presentation_is_consistent_across_lists_and_icon_modes() {
    use crate::directory_tree::TreeEntry;
    use crate::finder::{FinderEntryKind, FinderResult};
    use ratatui::layout::Margin;
    let mut theme = Theme::emergency_monochrome();
    theme.file_color_policy = crate::theme::FileColorPolicy::DeviconsRgb;
    theme.directory = Rgb::parse("#89B4FA").unwrap().into();
    theme.hidden = Rgb::parse("#6C7086").unwrap().into();
    let file_color = crate::icons::resolve(
        IconMode::NerdFont,
        "main.rs",
        crate::icons::EntryIconKind::File,
    )
    .color
    .unwrap();
    let entries = [
        ("main.rs", false, false),
        ("Source", true, false),
        (".hidden.rs", false, true),
    ]
    .map(|(name, is_directory, is_hidden)| DirectoryEntry {
        name: name.into(),
        path: name.into(),
        is_directory,
        is_hidden,
        is_parent: false,
    });
    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        for colored in [false, true] {
            for context in ["panel", "directory", "archive", "finder", "tree", "edit"] {
                let mut state = AppState::new(
                    ".".into(),
                    "test".into(),
                    TerminalSize {
                        columns: 100,
                        rows: 40,
                    },
                    WorkspaceOptions {
                        icon_mode: mode,
                        color_file_names: colored,
                        ..WorkspaceOptions::default()
                    },
                );
                state.workspace.panels[0].entries = entries.to_vec();
                let panel_layout = resolve_layout_with_preview(
                    state.terminal_size.area(),
                    Some(PanelId::Right),
                    state.preview_options.media_percent,
                );
                let area = match context {
                    "directory" | "archive" => {
                        state.workspace.preview_visible = true;
                        if context == "directory" {
                            state.preview.directory = LoadState::Ready(DirectoryPreview {
                                entries: entries
                                    .iter()
                                    .map(|entry| DirectoryPreviewEntry {
                                        name: entry.name.clone(),
                                        is_directory: entry.is_directory,
                                        is_hidden: entry.is_hidden,
                                    })
                                    .collect(),
                                truncated: false,
                            });
                        } else {
                            state.preview.archive = LoadState::Ready(ArchivePreview {
                                entries: entries
                                    .iter()
                                    .map(|entry| ArchivePreviewEntry {
                                        path: entry.name.clone(),
                                        is_directory: entry.is_directory,
                                    })
                                    .collect(),
                                truncated: false,
                            });
                        }
                        panel_layout.preview.unwrap().media
                    }
                    "finder" => {
                        state.finder.open(PanelId::Left, ".".into());
                        state.finder.results = entries
                            .iter()
                            .map(|entry| FinderResult {
                                absolute_path: entry.path.clone(),
                                relative_path: entry.name.clone(),
                                is_hidden: entry.is_hidden,
                                kind: if entry.is_directory {
                                    FinderEntryKind::Directory
                                } else {
                                    FinderEntryKind::File
                                },
                            })
                            .collect();
                        finder::resolve_layout(state.terminal_size.area()).results_inner
                    }
                    "tree" => {
                        state.directory_tree.open = true;
                        state.directory_tree.entries = entries
                            .iter()
                            .map(|entry| TreeEntry {
                                name: entry.name.clone(),
                                path: entry.path.clone(),
                                is_directory: entry.is_directory,
                                is_hidden: entry.is_hidden,
                                depth: 1,
                                expanded: false,
                                loading: false,
                            })
                            .collect();
                        finder::modal_area(state.terminal_size.area()).inner(Margin::new(1, 1))
                    }
                    "edit" => {
                        state
                            .directory_edit
                            .begin(PanelId::Left, ".".into(), &entries, 0);
                        panel_layout.left_inner
                    }
                    _ => panel_layout.left_inner,
                };
                let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
                terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
                let buffer = terminal.backend().buffer();
                let find = |text: &str| {
                    for y in area.y..area.bottom() {
                        for x in area.x..area.right() {
                            let row: String =
                                (x..area.right()).map(|x| buffer[(x, y)].symbol()).collect();
                            if row.starts_with(text) {
                                return (x, y);
                            }
                        }
                    }
                    panic!("missing {text} in {context}, {mode:?}");
                };
                let file = find("main.rs");
                if mode != IconMode::None {
                    let icon =
                        crate::icons::resolve(mode, "main.rs", crate::icons::EntryIconKind::File);
                    assert_eq!(buffer[(file.0 - 2, file.1)].symbol(), icon.glyph);
                    assert_eq!(buffer[(file.0 - 2, file.1)].fg, file_color.into());
                }
                assert_eq!(
                    buffer[file].fg,
                    if colored {
                        file_color.into()
                    } else {
                        theme.text
                    },
                    "{context}, {mode:?}"
                );
                let folder = find(if mode == IconMode::None || context == "edit" {
                    "Source/"
                } else {
                    "Source"
                });
                assert_eq!(buffer[folder].fg, theme.directory);
                if context != "archive" {
                    assert_eq!(buffer[find(".hidden.rs")].fg, theme.hidden);
                }
                if mode == IconMode::None && context == "panel" {
                    assert_eq!(file.0, area.x + 2);
                }
                if mode == IconMode::None && context == "edit" {
                    use ratatui::backend::Backend;
                    assert_eq!(file.0, area.x);
                    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
                    assert_eq!(cursor.x, area.x + "main.rs".len() as u16);
                }
            }
        }
    }
}

#[test]
fn view_does_not_mutate_application_state() {
    let state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    let before = state.clone();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| draw(frame, &state, &Theme::emergency_monochrome()))
        .unwrap();
    assert_eq!(state, before);
}

#[test]
fn active_and_inactive_panels_use_distinct_styles_and_only_active_selection() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).directory = PathBuf::from("long unicode 目录");
        state.workspace.panel_mut(panel).entries = vec![DirectoryEntry {
            path: PathBuf::from("文件.txt"),
            name: "文件.txt".into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        }];
    }
    state.workspace.icon_mode = IconMode::Unicode;
    let active = Rgb {
        red: 1,
        green: 2,
        blue: 3,
    };
    let inactive = Rgb {
        red: 4,
        green: 5,
        blue: 6,
    };
    let selected = Rgb {
        red: 7,
        green: 8,
        blue: 9,
    };
    let mut theme = Theme::emergency_monochrome();
    theme.active_border = active.into();
    theme.inactive_border = inactive.into();
    theme.selected = selected.into();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 1)].fg, active.into());
    assert_eq!(buffer[(40, 1)].fg, inactive.into());
    assert_eq!(buffer[(1, 1)].bg, selected.into());
    assert_eq!(buffer[(41, 1)].bg, theme.base);
    assert!(buffer.content().iter().any(|cell| cell.symbol() == "🗎"));

    theme.borderless_active_title_background = Rgb {
        red: 10,
        green: 11,
        blue: 12,
    }
    .into();
    theme.borderless_inactive_title_background = Rgb {
        red: 13,
        green: 14,
        blue: 15,
    }
    .into();
    theme.borderless_preview_title_background = Rgb {
        red: 16,
        green: 17,
        blue: 18,
    }
    .into();
    theme.borderless_title_left = "".into();
    theme.borderless_title_right = "".into();
    state.workspace.icon_mode = IconMode::NerdFont;
    state.workspace.panel_style = crate::workspace::PanelStyle::Borderless;
    let borderless = resolve_layout_with_styles(
        state.terminal_size.area(),
        None,
        state.preview_options.media_percent,
        state.status_line_options.style,
        state.workspace.panel_style,
    );
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "");
    assert_eq!(buffer[(0, 0)].fg, theme.borderless_active_title_background);
    assert_eq!(buffer[(1, 0)].bg, theme.borderless_active_title_background);
    assert_eq!(buffer[(borderless.left_panel.right() - 1, 0)].symbol(), "");
    assert_eq!(buffer[(borderless.panel_gap.x, 0)].symbol(), " ");
    assert_eq!(buffer[(borderless.right_panel.x, 0)].symbol(), "");
    assert_eq!(
        buffer[(borderless.right_panel.x + 1, 0)].bg,
        theme.borderless_inactive_title_background
    );

    state.workspace.preview_visible = true;
    state.preview.title = "preview.png".into();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(borderless.right_panel.x + 1, 0)].bg,
        theme.borderless_preview_title_background
    );
    assert_eq!(
        buffer[(borderless.right_panel.x + 1, 1)].bg,
        theme.preview_background
    );

    state.workspace.preview_visible = false;
    state.workspace.panel_style = crate::workspace::PanelStyle::Compact;
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(borderless.panel_gap.x, 0)].symbol(), " ");
    assert_eq!(buffer[(borderless.panel_gap.x, 1)].symbol(), "│");
    assert_eq!(
        buffer[(borderless.panel_gap.x, 1)].fg,
        theme.inactive_border
    );
}
