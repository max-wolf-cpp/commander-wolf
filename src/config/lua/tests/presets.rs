use super::*;

#[test]
fn shipped_themes_and_styles_resolve_independently() {
    use crate::theme::FileColorPolicy;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::DeviconsRgb
    );
    assert_eq!(runtime.theme().unwrap().border_set().top_left, "╭");
    for (name, panels, footer, icons, names) in [
        (
            "nvim",
            PanelStyle::Borderless,
            UiStyle::Borderless,
            IconMode::NerdFont,
            false,
        ),
        (
            "modern",
            PanelStyle::Framed,
            UiStyle::Framed,
            IconMode::NerdFont,
            false,
        ),
        (
            "classic",
            PanelStyle::Framed,
            UiStyle::Borderless,
            IconMode::None,
            true,
        ),
    ] {
        runtime.use_style(name).unwrap();
        assert_eq!(runtime.workspace_options().panel_style, panels);
        assert_eq!(runtime.status_line_options().style, footer);
        assert_eq!(runtime.workspace_options().icon_mode, icons);
        assert_eq!(runtime.workspace_options().color_file_names, names);
    }
    runtime.use_theme("far").unwrap();
    let far = runtime.theme().unwrap();
    assert_eq!(far.panel_border_set().top_left, "╔");
    assert!(!far.decorations);
    assert_eq!(far.file_color_policy, FileColorPolicy::NearestPalette);
    assert_eq!(
        runtime
            .syntax_theme()
            .unwrap()
            .color(crate::syntax::SyntaxCategory::Keyword, far.text),
        ratatui::style::Color::Rgb(0, 255, 255)
    );
    runtime.use_theme("catppuccin_mocha").unwrap();
    assert_eq!(runtime.display_style().unwrap().icons, IconMode::None);
}

#[test]
fn shipped_far_uses_classic_panel_colors_and_a_black_footer() {
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("far").unwrap();
    let theme = runtime.theme().unwrap();
    let blue = Color::Rgb(0, 0, 128);
    let cyan = Color::Rgb(0, 128, 128);
    let light_cyan = Color::Rgb(0, 255, 255);
    let black = Color::Rgb(0, 0, 0);

    assert_eq!(theme.base, blue);
    assert_eq!(theme.text, light_cyan);
    assert_eq!(theme.active_border, light_cyan);
    assert_eq!(theme.inactive_border, light_cyan);
    assert_eq!(theme.directory, Color::Rgb(255, 255, 255));
    assert_eq!(theme.hidden, cyan);
    assert_eq!((theme.selected_text, theme.selected), (black, cyan));
    assert_eq!(
        (theme.inactive_selected_text, theme.inactive_selected),
        (None, None)
    );
    assert_eq!(theme.active_title, Color::Rgb(255, 255, 0));
    assert_eq!(theme.framed_active_title_background, None);
    assert_eq!(theme.framed_active_title_text, None);
    assert_eq!(
        (
            theme.borderless_title_text,
            theme.borderless_active_title_background
        ),
        (Color::Rgb(255, 255, 0), blue)
    );
    assert_eq!(
        (
            theme.borderless_inactive_title_text,
            theme.borderless_inactive_title_background
        ),
        (Some(light_cyan), blue)
    );
    assert_eq!(
        (
            theme.preview_title_text,
            theme.borderless_preview_title_background
        ),
        (Some(Color::Rgb(255, 0, 255)), blue)
    );
    assert_eq!(theme.error, Color::Rgb(255, 0, 0));
    assert_eq!(theme.dialog_error, theme.error);
    assert_eq!(theme.directory_edit_delete, theme.error);
    assert_eq!(theme.status_background, black);
    assert_eq!(theme.compact_status_background, black);
    assert_eq!(theme.command_background, black);

    for style in [UiStyle::Framed, UiStyle::Borderless, UiStyle::Compact] {
        let mut state = crate::app::AppState::new(
            ".".into(),
            "test".into(),
            crate::terminal::TerminalSize {
                columns: 80,
                rows: 24,
            },
            runtime.workspace_options(),
        );
        state.status_line_options.style = style;
        let layout = crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            None,
            state.preview_options.media_percent,
            style,
            state.workspace.panel_style,
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        for area in [
            layout.bottom_frame,
            layout.bottom_separator,
            layout.command_line,
        ] {
            for y in area.y..area.bottom() {
                for x in area.x..area.right() {
                    assert_eq!(buffer[(x, y)].bg, black, "{style:?} ({x}, {y})");
                }
            }
        }
        assert_eq!(
            buffer[(layout.status_line.right() - 1, layout.status_line.y)].bg,
            black
        );
    }
}

#[test]
fn shipped_far_highlights_only_the_active_panels_cursor_and_path_title() {
    use crate::app::AppState;
    use crate::terminal::TerminalSize;
    use crate::workspace::{DirectoryEntry, PanelId};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("far").unwrap();
    let theme = runtime.theme().unwrap();
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        runtime.workspace_options(),
    );
    state.workspace.icon_mode = IconMode::None;
    for (id, directory, selected) in [(PanelId::Left, "left", 0), (PanelId::Right, "right", 1)] {
        let panel = state.workspace.panel_mut(id);
        panel.directory = directory.into();
        panel.selected = selected;
        panel.entries = ["first.txt", "second.txt"]
            .map(|name| DirectoryEntry {
                path: Path::new(directory).join(name),
                name: name.into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            })
            .to_vec();
    }
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    for style in [
        PanelStyle::Framed,
        PanelStyle::Borderless,
        PanelStyle::Compact,
    ] {
        state.workspace.panel_style = style;
        let layout = crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            None,
            state.preview_options.media_percent,
            state.status_line_options.style,
            style,
        );
        for active in [PanelId::Left, PanelId::Right, PanelId::Left] {
            state.workspace.active_panel = active;
            terminal
                .draw(|frame| crate::ui::draw(frame, &state, &theme))
                .unwrap();
            let buffer = terminal.backend().buffer();
            for (id, panel_area, inner) in [
                (PanelId::Left, layout.left_panel, layout.left_inner),
                (PanelId::Right, layout.right_panel, layout.right_inner),
            ] {
                let active = id == active;
                let selected = state.workspace.panel(id).selected;
                let row_background = if active { theme.selected } else { theme.base };
                for x in inner.x..inner.right() {
                    assert_eq!(
                        buffer[(x, inner.y + selected as u16)].bg,
                        row_background,
                        "{style:?} {id:?}"
                    );
                }
                let title = &buffer[(panel_area.x + 5, panel_area.y)];
                assert_eq!(title.bg, theme.base, "{style:?} {id:?}");
                assert_eq!(
                    title.fg,
                    if active {
                        theme.active_title
                    } else {
                        theme.inactive_title
                    },
                    "{style:?} {id:?}"
                );
                if style == PanelStyle::Framed {
                    assert_eq!(buffer[(panel_area.x + 3, panel_area.y)].bg, theme.base);
                }
            }
            assert_eq!(state.workspace.panel(PanelId::Left).selected, 0);
            assert_eq!(state.workspace.panel(PanelId::Right).selected, 1);
        }

        state.workspace.preview_visible = true;
        state.preview.title = "stats.zip".into();
        let layout = crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            state.workspace.preview_panel(),
            state.preview_options.media_percent,
            state.status_line_options.style,
            style,
        );
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let panel = layout.preview.unwrap().panel;
        let buffer = terminal.backend().buffer();
        let title_x = (panel.x..panel.right())
            .find(|&x| buffer[(x, panel.y)].symbol() == "s")
            .expect("preview title is visible");
        assert_eq!(
            buffer[(title_x, panel.y)].fg,
            ratatui::style::Color::Rgb(255, 0, 255)
        );
        assert_eq!(buffer[(title_x, panel.y)].bg, theme.base);
        state.workspace.preview_visible = false;
    }
}

#[test]
fn shipped_far_colors_dialog_surfaces_without_changing_other_floats() {
    use crate::app::{AppMode, AppState};
    use crate::dialogs::{DialogDisplay, DialogKind, RenamePrompt};
    use crate::file_operations::{FileOperationKind, progress::OperationProgress};
    use crate::terminal::TerminalSize;
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("far").unwrap();
    let theme = runtime.theme().unwrap();
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        runtime.workspace_options(),
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
    let gray = Color::Rgb(192, 192, 192);
    let red = Color::Rgb(128, 0, 0);
    let black = Color::Rgb(0, 0, 0);
    let white = Color::Rgb(255, 255, 255);
    let cyan = Color::Rgb(0, 128, 128);
    let yellow = Color::Rgb(255, 255, 0);

    state.mode = AppMode::Confirm;
    for (kind, background, foreground) in [
        (DialogKind::Info, gray, black),
        (DialogKind::Warning, red, white),
        (DialogKind::Error, red, white),
    ] {
        state.dialog_kind = kind;
        let shortcut = if kind == DialogKind::Info {
            cyan
        } else {
            yellow
        };
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let cells: Vec<_> = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.bg == background)
            .collect();
        assert!(
            cells.len() >= 76 * 5,
            "{kind:?}: full dialog surface is painted"
        );
        for cell in cells.iter().filter(|cell| cell.symbol() != " ") {
            assert!(
                cell.fg == foreground || cell.fg == shortcut,
                "{kind:?}: {}",
                cell.symbol()
            );
        }
        assert_dialog_shortcuts(
            terminal.backend().buffer(),
            "Yes   No   Cancel",
            &["Y", "N", "C"],
            (background, foreground, shortcut),
        );
    }

    state.mode = AppMode::Normal;
    state.rename_prompt = Some(RenamePrompt::new(
        "old.txt".into(),
        "new.txt".into(),
        DialogDisplay::Float,
    ));
    for error in [None, Some("Invalid name".to_owned())] {
        state.rename_prompt.as_mut().unwrap().error = error.clone();
        let (background, foreground) = if error.is_some() {
            (red, white)
        } else {
            (gray, black)
        };
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let cells: Vec<_> = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.bg == background)
            .collect();
        assert!(cells.len() >= 60 * 8);
        for cell in cells.iter().filter(|cell| cell.symbol() != " ") {
            let shortcut = if error.is_some() { yellow } else { cyan };
            assert!(
                cell.fg == foreground || cell.fg == shortcut,
                "Rename: {}",
                cell.symbol()
            );
        }
        assert_dialog_shortcuts(
            terminal.backend().buffer(),
            "Enter Rename   Esc Cancel   Ctrl+A Select all",
            &["Enter", "Esc", "Ctrl+A"],
            (
                background,
                foreground,
                if error.is_some() { yellow } else { cyan },
            ),
        );
    }

    state.rename_prompt = None;
    state.operation_progress = Some(OperationProgress::new(FileOperationKind::Copy));
    terminal
        .draw(|frame| crate::ui::draw(frame, &state, &theme))
        .unwrap();
    let cells: Vec<_> = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .filter(|cell| cell.bg == gray)
        .collect();
    assert!(cells.len() >= 80 * 8);
    for cell in cells.iter().filter(|cell| cell.symbol() != " ") {
        assert!(
            cell.fg == black || cell.fg == cyan,
            "Progress: {}",
            cell.symbol()
        );
    }
    assert_dialog_shortcuts(
        terminal.backend().buffer(),
        "Cancel  (Esc / Ctrl+C)",
        &["C"],
        (gray, black, cyan),
    );

    state.operation_progress = None;
    assert_eq!(theme.float_background, Color::Rgb(0, 0, 128));
    state
        .finder
        .open(crate::workspace::PanelId::Left, ".".into());
    terminal
        .draw(|frame| crate::ui::draw(frame, &state, &theme))
        .unwrap();
    assert!(
        !terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.bg == gray || cell.bg == red)
    );
    state.finder.close();
    state.directory_tree.open = true;
    terminal
        .draw(|frame| crate::ui::draw(frame, &state, &theme))
        .unwrap();
    assert!(
        !terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.bg == gray || cell.bg == red)
    );
    state.directory_tree.open = false;
    state.viewer.open("image.png".into(), "image.png".into());
    terminal
        .draw(|frame| crate::ui::draw(frame, &state, &theme))
        .unwrap();
    assert_eq!(
        terminal.backend().buffer()[(50, 15)].bg,
        theme.float_background
    );
    assert!(
        !terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.bg == gray || cell.bg == red)
    );
}

fn assert_dialog_shortcuts(
    buffer: &ratatui::buffer::Buffer,
    label: &str,
    keys: &[&str],
    (background, foreground, shortcut): (
        ratatui::style::Color,
        ratatui::style::Color,
        ratatui::style::Color,
    ),
) {
    assert!(label.is_ascii());
    let (x, y) = (buffer.area.y..buffer.area.bottom())
        .find_map(|y| {
            (buffer.area.x..buffer.area.right().saturating_sub(label.len() as u16)).find_map(|x| {
                label
                    .char_indices()
                    .all(|(offset, _)| {
                        buffer[(x + offset as u16, y)].symbol() == &label[offset..offset + 1]
                    })
                    .then_some((x, y))
            })
        })
        .expect("dialog action label is visible");
    let key_ranges: Vec<_> = keys
        .iter()
        .map(|key| {
            let start = label.find(key).expect("key is present in the label");
            start..start + key.len()
        })
        .collect();

    for (offset, character) in label.char_indices() {
        let cell = &buffer[(x + offset as u16, y)];
        assert_eq!(cell.bg, background, "{label}: {offset}");
        if character == ' ' {
            continue;
        }

        let is_key = key_ranges.iter().any(|range| range.contains(&offset));
        assert_eq!(
            cell.fg,
            if is_key { shortcut } else { foreground },
            "{label}: {offset}"
        );
        if is_key {
            assert!(cell.modifier.contains(ratatui::style::Modifier::BOLD));
        }
    }
}

#[test]
fn shipped_far_distinguishes_footer_modes_and_preserves_the_normal_badge() {
    use crate::app::{AppMode, AppState};
    use crate::status_line::ModePosition;
    use crate::terminal::TerminalSize;
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("far").unwrap();
    let theme = runtime.theme().unwrap();
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        runtime.workspace_options(),
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();

    for style in [UiStyle::Framed, UiStyle::Borderless, UiStyle::Compact] {
        state.status_line_options.style = style;
        let layout = crate::ui::resolve_layout_with_styles(
            state.terminal_size.area(),
            None,
            state.preview_options.media_percent,
            style,
            state.workspace.panel_style,
        );
        for position in [ModePosition::Left, ModePosition::Right] {
            state.status_line_options.mode_position = position;
            for (mode, label, color) in [
                (AppMode::Normal, "NORMAL", Color::Rgb(255, 255, 255)),
                (AppMode::Insert, "INSERT", Color::Rgb(0, 255, 0)),
                (AppMode::Search, "SEARCH", Color::Rgb(0, 255, 255)),
                (AppMode::Visual, "VISUAL", Color::Rgb(255, 0, 255)),
                (AppMode::Command, "COMMAND", Color::Rgb(255, 255, 0)),
            ] {
                state.mode = mode;
                terminal
                    .draw(|frame| crate::ui::draw(frame, &state, &theme))
                    .unwrap();
                let compact = style != UiStyle::Framed;
                let width = label.len() as u16 + if compact { 2 } else { 0 };
                let x = match position {
                    ModePosition::Left => layout.status_line.x,
                    ModePosition::Right => layout.status_line.right() - width,
                };
                let x = x + u16::from(compact);
                let cell = &terminal.backend().buffer()[(x, layout.status_line.y)];
                assert_eq!(cell.symbol(), &label[..1], "{style:?} {position:?}");
                let (foreground, background) = if compact {
                    (
                        Color::Rgb(0, 0, 0),
                        if mode == AppMode::Normal {
                            Color::Rgb(0, 128, 128)
                        } else {
                            color
                        },
                    )
                } else {
                    (color, Color::Rgb(0, 0, 0))
                };
                assert_eq!(
                    (cell.fg, cell.bg),
                    (foreground, background),
                    "{style:?} {label}"
                );
            }
        }
    }
}

#[test]
fn shipped_nvim_renders_borderless_panels_plain_names_and_rounded_floats() {
    use crate::app::AppState;
    use crate::terminal::TerminalSize;
    use crate::workspace::{DirectoryEntry, PanelId};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_style("nvim").unwrap();
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 100,
            rows: 30,
        },
        runtime.workspace_options(),
    );
    state.status_line_options = runtime.status_line_options();
    state.workspace.panels[0].entries = ["first.txt", "main.rs", "photo.png"]
        .map(|name| DirectoryEntry {
            path: name.into(),
            name: name.into(),
            is_directory: false,
            is_parent: false,
            is_hidden: false,
        })
        .to_vec();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    for name in ["far", "catppuccin_mocha"] {
        runtime.use_theme(name).unwrap();
        let theme = runtime.theme().unwrap();
        assert_eq!(theme.border_set().top_left, "╭");
        terminal
            .draw(|frame| crate::ui::draw(frame, &state, &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(!buffer.content().iter().any(|cell| {
            matches!(cell.symbol(), "╭" | "╮" | "╰" | "╯" | "┌" | "┐" | "└" | "┘")
        }));
        for file in ["main.rs", "photo.png"] {
            let width = file.len() as u16;
            let position = (1..27)
                .find_map(|y| {
                    (0..50 - width).find_map(|x| {
                        let text: String =
                            (x..x + width).map(|x| buffer[(x, y)].symbol()).collect();
                        (text == file).then_some((x, y))
                    })
                })
                .expect("unselected file name is visible");
            assert_eq!(buffer[position].fg, theme.text, "{name}: {file}");
        }
    }

    state.finder.open(PanelId::Left, ".".into());
    let theme = runtime.theme().unwrap();
    terminal
        .draw(|frame| crate::ui::draw(frame, &state, &theme))
        .unwrap();
    let buffer = terminal.backend().buffer();
    for corner in ["╭", "╮", "╰", "╯"] {
        assert!(buffer.content().iter().any(|cell| cell.symbol() == corner));
    }

    runtime
        .lua
        .load("wolf.style.color_file_names = true; wolf.style.frames.border = 'double'")
        .exec()
        .unwrap();
    assert!(runtime.workspace_options().color_file_names);
    assert_eq!(runtime.theme().unwrap().border_set().top_left, "╔");
}

#[test]
fn shipped_catppuccin_flavors_use_matching_ui_and_syntax_palettes() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    let completions = runtime
        .command_completions(":theme ")
        .into_iter()
        .map(|item| item.name)
        .collect::<Vec<_>>();
    for (name, base, selected, keyword, light_icons) in [
        ("catppuccin_latte", "#eff1f5", "#ccd0da", "#8839ef", true),
        ("catppuccin_frappe", "#303446", "#414559", "#ca9ee6", false),
        (
            "catppuccin_macchiato",
            "#24273a",
            "#363a4f",
            "#c6a0f6",
            false,
        ),
        ("catppuccin_mocha", "#1e1e2e", "#313244", "#cba6f7", false),
    ] {
        assert!(completions.contains(&name.to_owned()), "{name}");
        runtime.use_theme(name).unwrap();
        let theme = runtime.theme().unwrap();
        assert_eq!(theme.base, Rgb::parse(base).unwrap().into(), "{name}");
        assert_eq!(
            theme.selected,
            Rgb::parse(selected).unwrap().into(),
            "{name}"
        );
        assert_eq!(theme.light_file_icons, light_icons, "{name}");
        assert_eq!(
            runtime
                .syntax_theme()
                .unwrap()
                .color(SyntaxCategory::Keyword, theme.text),
            Rgb::parse(keyword).unwrap().into(),
            "{name}"
        );
    }
}

#[test]
fn shipped_dracula_uses_its_ui_and_syntax_palette() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .command_completions(":theme ")
            .iter()
            .any(|item| item.name == "dracula")
    );
    runtime.use_theme("dracula").unwrap();
    let theme = runtime.theme().unwrap();
    assert_eq!(theme.base, Rgb::parse("#282A36").unwrap().into());
    assert_eq!(theme.directory, Rgb::parse("#BD93F9").unwrap().into());
    assert_eq!(theme.active_border, theme.directory);
    assert_eq!(theme.selected, Rgb::parse("#424450").unwrap().into());
    assert!(!theme.light_file_icons);
    let syntax = runtime.syntax_theme().unwrap();
    for (category, color) in [
        (SyntaxCategory::Keyword, "#FF79C6"),
        (SyntaxCategory::String, "#F1FA8C"),
        (SyntaxCategory::Function, "#50FA7B"),
    ] {
        assert_eq!(
            syntax.color(category, theme.text),
            Rgb::parse(color).unwrap().into()
        );
    }
}

#[test]
fn shipped_tokyonight_night_uses_matching_ui_and_syntax() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .command_completions(":theme ")
            .iter()
            .any(|item| item.name == "tokyonight_night")
    );
    runtime.use_theme("tokyonight_night").unwrap();
    let theme = runtime.theme().unwrap();
    assert_eq!(theme.base, Rgb::parse("#1a1b26").unwrap().into());
    assert_eq!(theme.directory, Rgb::parse("#7aa2f7").unwrap().into());
    assert_eq!(theme.active_border, theme.directory);
    assert_eq!(theme.selected, Rgb::parse("#292e42").unwrap().into());
    assert!(!theme.light_file_icons);
    assert_eq!(
        runtime
            .syntax_theme()
            .unwrap()
            .color(SyntaxCategory::Keyword, theme.text),
        Rgb::parse("#9d7cd8").unwrap().into()
    );
}

#[test]
fn shipped_gruvbox_modes_use_matching_ui_syntax_and_icons() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    let completions = runtime
        .command_completions(":theme ")
        .into_iter()
        .map(|item| item.name)
        .collect::<Vec<_>>();
    for (name, base, selected, directory, keyword, light_icons) in [
        (
            "gruvbox_dark",
            "#282828",
            "#3c3836",
            "#b8bb26",
            "#fb4934",
            false,
        ),
        (
            "gruvbox_light",
            "#fbf1c7",
            "#ebdbb2",
            "#79740e",
            "#9d0006",
            true,
        ),
    ] {
        assert!(completions.contains(&name.to_owned()), "{name}");
        runtime.use_theme(name).unwrap();
        let theme = runtime.theme().unwrap();
        assert_eq!(theme.base, Rgb::parse(base).unwrap().into(), "{name}");
        assert_eq!(
            theme.selected,
            Rgb::parse(selected).unwrap().into(),
            "{name}"
        );
        assert_eq!(
            theme.directory,
            Rgb::parse(directory).unwrap().into(),
            "{name}"
        );
        assert_eq!(theme.active_border, theme.directory, "{name}");
        assert_eq!(theme.light_file_icons, light_icons, "{name}");
        assert_eq!(
            runtime
                .syntax_theme()
                .unwrap()
                .color(SyntaxCategory::Keyword, theme.text),
            Rgb::parse(keyword).unwrap().into(),
            "{name}"
        );
    }
}

#[test]
fn shipped_nord_uses_matching_ui_and_syntax() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .command_completions(":theme ")
            .iter()
            .any(|item| item.name == "nord")
    );
    runtime.use_theme("nord").unwrap();
    let theme = runtime.theme().unwrap();
    assert_eq!(theme.base, Rgb::parse("#2e3440").unwrap().into());
    assert_eq!(theme.directory, Rgb::parse("#88c0d0").unwrap().into());
    assert_eq!(theme.active_border, theme.directory);
    assert_eq!(theme.selected, Rgb::parse("#3b4252").unwrap().into());
    assert!(!theme.light_file_icons);
    let syntax = runtime.syntax_theme().unwrap();
    for (category, color) in [
        (SyntaxCategory::Keyword, "#81a1c1"),
        (SyntaxCategory::String, "#a3be8c"),
        (SyntaxCategory::Function, "#88c0d0"),
    ] {
        assert_eq!(
            syntax.color(category, theme.text),
            Rgb::parse(color).unwrap().into()
        );
    }
}

#[test]
fn shipped_one_dark_uses_matching_ui_and_syntax() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .command_completions(":theme ")
            .iter()
            .any(|item| item.name == "one_dark")
    );
    runtime.use_theme("one_dark").unwrap();
    let theme = runtime.theme().unwrap();
    assert_eq!(theme.base, Rgb::parse("#282c34").unwrap().into());
    assert_eq!(theme.directory, Rgb::parse("#61afef").unwrap().into());
    assert_eq!(theme.active_border, theme.directory);
    assert_eq!(theme.selected, Rgb::parse("#31353f").unwrap().into());
    assert!(!theme.light_file_icons);
    let syntax = runtime.syntax_theme().unwrap();
    for (category, color) in [
        (SyntaxCategory::Keyword, "#c678dd"),
        (SyntaxCategory::String, "#98c379"),
        (SyntaxCategory::Function, "#61afef"),
        (SyntaxCategory::Variable, "#e06c75"),
    ] {
        assert_eq!(
            syntax.color(category, theme.text),
            Rgb::parse(color).unwrap().into()
        );
    }
}

#[test]
fn shipped_solarized_modes_use_matching_ui_syntax_and_icons() {
    use crate::syntax::SyntaxCategory;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    let completions = runtime
        .command_completions(":theme ")
        .into_iter()
        .map(|item| item.name)
        .collect::<Vec<_>>();
    for (name, base, text, selected, comment, light_icons) in [
        (
            "solarized_dark",
            "#002b36",
            "#839496",
            "#073642",
            "#586e75",
            false,
        ),
        (
            "solarized_light",
            "#fdf6e3",
            "#657b83",
            "#eee8d5",
            "#93a1a1",
            true,
        ),
    ] {
        assert!(completions.contains(&name.to_owned()), "{name}");
        runtime.use_theme(name).unwrap();
        let theme = runtime.theme().unwrap();
        assert_eq!(theme.base, Rgb::parse(base).unwrap().into(), "{name}");
        assert_eq!(theme.text, Rgb::parse(text).unwrap().into(), "{name}");
        assert_eq!(
            theme.selected,
            Rgb::parse(selected).unwrap().into(),
            "{name}"
        );
        assert_eq!(theme.directory, Rgb::parse("#268bd2").unwrap().into());
        assert_eq!(theme.active_border, theme.directory);
        assert_eq!(
            theme.search_match_background,
            Rgb::parse("#b58900").unwrap().into()
        );
        assert_eq!(
            theme.search_current_match_background,
            Rgb::parse("#cb4b16").unwrap().into()
        );
        assert_eq!(theme.light_file_icons, light_icons, "{name}");
        let syntax = runtime.syntax_theme().unwrap();
        for (category, color) in [
            (SyntaxCategory::Comment, comment),
            (SyntaxCategory::Keyword, "#859900"),
            (SyntaxCategory::String, "#2aa198"),
            (SyntaxCategory::Type, "#b58900"),
            (SyntaxCategory::Function, "#268bd2"),
        ] {
            assert_eq!(
                syntax.color(category, theme.text),
                Rgb::parse(color).unwrap().into(),
                "{name}"
            );
        }
    }
}

#[test]
fn shipped_fixed_themes_ignore_terminal_palette() {
    use crate::terminal_appearance::Scheme;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    {
        let draft = runtime.draft.borrow();
        for name in [
            "far",
            "catppuccin_latte",
            "catppuccin_frappe",
            "catppuccin_macchiato",
            "catppuccin_mocha",
            "dracula",
            "gruvbox_dark",
            "gruvbox_light",
            "nord",
            "one_dark",
            "solarized_dark",
            "solarized_light",
            "tokyonight_night",
        ] {
            assert!(
                draft.themes[name]
                    .iter()
                    .filter(|(role, _)| *role != "file_color_policy")
                    .all(|(_, color)| color.starts_with('#'))
            );
            assert!(draft.syntax_themes[name].parent.is_none());
            assert!(
                draft.syntax_themes[name]
                    .fields
                    .values()
                    .all(|color| color.starts_with('#'))
            );
        }
    }
    for name in [
        "far",
        "catppuccin_latte",
        "catppuccin_frappe",
        "catppuccin_macchiato",
        "catppuccin_mocha",
        "dracula",
        "gruvbox_dark",
        "gruvbox_light",
        "nord",
        "one_dark",
        "solarized_dark",
        "solarized_light",
        "tokyonight_night",
    ] {
        runtime.use_theme(name).unwrap();
        runtime.update_terminal_appearance(TerminalAppearance {
            scheme: Scheme::Dark,
            background: Some(Rgb {
                red: 0,
                green: 0,
                blue: 0,
            }),
            palette: Some(
                [Rgb {
                    red: 0,
                    green: 0,
                    blue: 0,
                }; 16],
            ),
        });
        let dark_ui = runtime.theme().unwrap();
        let dark_syntax = runtime.syntax_theme().unwrap();
        if name == "far" {
            let source =
                crate::icons::resolve(IconMode::None, "main.rs", crate::icons::EntryIconKind::File);
            let color = source.file_label_color(
                "main.rs",
                crate::icons::EntryIconKind::File,
                &dark_ui,
                true,
            );
            assert!(
                crate::theme::FAR_PALETTE
                    .iter()
                    .any(|swatch| ratatui::style::Color::from(*swatch) == color)
            );
            assert_eq!(
                source.themed_display_color(crate::icons::EntryIconKind::File, &dark_ui),
                color
            );
        }
        runtime.update_terminal_appearance(TerminalAppearance {
            scheme: Scheme::Light,
            background: Some(Rgb {
                red: 255,
                green: 255,
                blue: 255,
            }),
            palette: Some(
                [Rgb {
                    red: 255,
                    green: 255,
                    blue: 255,
                }; 16],
            ),
        });
        assert_eq!(runtime.theme().unwrap(), dark_ui, "UI theme {name}");
        assert_eq!(
            runtime.syntax_theme().unwrap(),
            dark_syntax,
            "syntax theme {name}"
        );
    }
}
