use ratatui::style::Color;

use super::*;
use crate::theme::FileColorPolicy;

#[test]
fn original_devicons_policy_uses_source_rgb_even_with_terminal_default_text() {
    let mut theme = Theme::emergency_monochrome();
    theme.text = Color::Reset;
    theme.unknown_file_icon = Color::Reset;
    theme.file_color_policy = FileColorPolicy::OriginalDeviconsRgb;
    let file = resolve_for_theme(IconMode::NerdFont, "main.rs", EntryIconKind::File, &theme);
    let source = Color::from(file.color.expect("known devicons color"));

    assert_eq!(
        file.file_label_color("main.rs", EntryIconKind::File, &theme, true),
        source
    );
    assert_eq!(
        file.themed_display_color(EntryIconKind::File, &theme),
        source
    );
}

#[test]
fn semantic_ansi_name_colors_work_even_without_icons() {
    let mut theme = Theme::emergency_monochrome();
    theme.file_color_policy = FileColorPolicy::SemanticAnsi;
    theme.directory = Color::Indexed(11);
    theme.text = Color::Indexed(15);

    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        let file = resolve(mode, "main.rs", EntryIconKind::File);
        assert_eq!(
            file.file_label_color("main.rs", EntryIconKind::File, &theme, true),
            Color::Indexed(14)
        );
        assert_eq!(
            file.file_label_color("main.rs", EntryIconKind::File, &theme, false),
            theme.text
        );

        let folder = resolve(mode, "Source", EntryIconKind::Directory);
        assert_eq!(
            folder.file_label_color("Source", EntryIconKind::Directory, &theme, true),
            theme.directory
        );
    }
}

#[test]
fn semantic_palette_name_colors_use_ui_rgb_even_without_icons() {
    let mut theme = Theme::emergency_monochrome();
    theme.file_color_policy = FileColorPolicy::SemanticPalette;
    theme.text = Color::Rgb(255, 255, 255);
    theme.directory = Color::Rgb(255, 255, 0);
    theme.mode_command = Color::Rgb(0, 255, 255);
    theme.hidden = Color::Rgb(192, 192, 192);

    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        let file = resolve(mode, "main.rs", EntryIconKind::File);
        assert_eq!(
            file.file_label_color("main.rs", EntryIconKind::File, &theme, true),
            Color::Rgb(0, 255, 255)
        );
        assert_eq!(
            file.file_label_color("unknown", EntryIconKind::File, &theme, true),
            Color::Rgb(192, 192, 192)
        );
        assert_eq!(
            file.file_label_color("main.rs", EntryIconKind::File, &theme, false),
            theme.text
        );
    }
}

#[test]
fn terminal_theme_uses_original_light_and_dark_devicons_colors() {
    use crate::terminal_appearance::{Scheme, TerminalAppearance};

    let dark_source = resolve(IconMode::NerdFont, "main.rs", EntryIconKind::File);
    for (scheme, background) in [
        (Scheme::Dark, Rgb::parse("#000000").unwrap()),
        (Scheme::Light, Rgb::parse("#FFFFFF").unwrap()),
    ] {
        let theme = crate::theme::derive_default_theme(TerminalAppearance {
            scheme,
            background: Some(background),
            palette: Some(crate::theme::FAR_PALETTE),
        });
        let file = resolve_for_theme(IconMode::NerdFont, "main.rs", EntryIconKind::File, &theme);

        assert_eq!(file.glyph, dark_source.glyph);
        assert_eq!(theme.light_file_icons, scheme == Scheme::Light);
        assert_eq!(file.color == dark_source.color, scheme == Scheme::Dark);
        let color = file.file_label_color("main.rs", EntryIconKind::File, &theme, true);
        assert_eq!(color, Color::from(file.color.unwrap()));
        assert_eq!(
            file.themed_display_color(EntryIconKind::File, &theme),
            color
        );
        assert!(theme.file_color_matches.contains_key(&file.color.unwrap()));
    }
}

#[test]
fn icon_modes_share_file_colors_and_none_has_no_icon_spacing() {
    let text: Color = Rgb::parse("#CDD6F4").unwrap().into();
    let directory: Color = Rgb::parse("#89B4FA").unwrap().into();
    for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
        for name in ["main.rs", "README.md", ".gitignore", "view.blade.php"] {
            let icon = resolve(mode, name, EntryIconKind::File);
            assert_eq!(
                icon.color,
                resolve(IconMode::NerdFont, name, EntryIconKind::File).color
            );
            assert_eq!(
                icon.label_color(EntryIconKind::File, directory, text, false),
                text
            );
            assert_eq!(
                icon.label_color(EntryIconKind::File, directory, text, true),
                icon.color.unwrap().into()
            );
        }
        for kind in [
            EntryIconKind::Parent,
            EntryIconKind::Directory,
            EntryIconKind::DirectoryOpen,
            EntryIconKind::File,
            EntryIconKind::New,
        ] {
            let icon = resolve(mode, "entry", kind);
            if mode == IconMode::None {
                assert_eq!(icon.glyph, "");
                assert_eq!(icon.spacer(), "");
            }
        }

        let unknown = resolve(mode, "unknown.no-wolf-extension", EntryIconKind::File);
        assert_eq!(
            unknown.label_color(EntryIconKind::File, directory, text, true),
            text
        );
    }

    assert_eq!(
        IconMode::None.label("Source", EntryIconKind::Directory),
        "Source/"
    );
    assert_eq!(
        IconMode::None.label("Source/", EntryIconKind::Directory),
        "Source/"
    );
    assert_eq!(
        IconMode::None.label("main.rs", EntryIconKind::File),
        "main.rs"
    );
    assert_eq!(IconMode::Unicode.decoration(""), "");
    assert_eq!(IconMode::None.decoration(""), "");
    assert_eq!(IconMode::NerdFont.decoration(""), "");
    assert_eq!(IconMode::parse("none").unwrap(), IconMode::None);
    assert_eq!(IconMode::parse("ascii").unwrap(), IconMode::None);
    assert!(IconMode::parse("emoji").is_err());
}

#[test]
fn nerd_font_icons_follow_filename_extension_and_compound_extension_tables() {
    assert_eq!(
        resolve(IconMode::NerdFont, ".GITIGNORE", EntryIconKind::File),
        ResolvedIcon {
            glyph: "",
            color: Some(Rgb::parse("#F54D27").unwrap()),
        }
    );
    assert_eq!(
        resolve(IconMode::NerdFont, "main.cpp", EntryIconKind::File),
        ResolvedIcon {
            glyph: "",
            color: Some(Rgb::parse("#519ABA").unwrap()),
        }
    );
    assert_eq!(
        resolve(IconMode::NerdFont, "view.blade.php", EntryIconKind::File),
        ResolvedIcon {
            glyph: "",
            color: Some(Rgb::parse("#F05340").unwrap()),
        }
    );
}

#[test]
fn unknown_files_and_non_nerd_modes_have_stable_fallbacks() {
    assert_eq!(
        resolve(IconMode::NerdFont, "unknown.wolf", EntryIconKind::File),
        ResolvedIcon {
            glyph: "",
            color: None,
        }
    );
    assert_eq!(
        resolve(IconMode::Unicode, "main.rs", EntryIconKind::File).glyph,
        "🗎"
    );
    assert_eq!(
        resolve(IconMode::None, "main.rs", EntryIconKind::File).glyph,
        ""
    );
}

#[test]
fn unicode_folder_and_document_glyphs_occupy_one_terminal_cell() {
    for kind in [
        EntryIconKind::Directory,
        EntryIconKind::DirectoryOpen,
        EntryIconKind::File,
    ] {
        let glyph = resolve(IconMode::Unicode, "entry", kind).glyph;
        assert_eq!(ratatui::text::Line::raw(glyph).width(), 1, "{kind:?}");
    }
}

#[test]
fn directories_are_blue_and_distinguish_open_and_closed_states() {
    for mode in [IconMode::NerdFont, IconMode::Unicode] {
        let closed = resolve(mode, "Source", EntryIconKind::Directory);
        let open = resolve(mode, "Source", EntryIconKind::DirectoryOpen);
        assert_ne!(closed.glyph, open.glyph);
    }

    assert_eq!(
        resolve(IconMode::NerdFont, "Source", EntryIconKind::Directory).glyph,
        ""
    );
    assert_eq!(
        resolve(IconMode::NerdFont, "Source", EntryIconKind::DirectoryOpen).glyph,
        ""
    );
    assert_eq!(
        resolve(IconMode::Unicode, "Source", EntryIconKind::Directory).glyph,
        "🗀"
    );
    assert_eq!(
        resolve(IconMode::Unicode, "Source", EntryIconKind::DirectoryOpen).glyph,
        "🗁"
    );

    let directory_color: Color = Rgb::parse("#89B4FA").unwrap().into();
    let unknown_file_color: Color = Rgb::parse("#6C7086").unwrap().into();
    let fallback: Color = Rgb::parse("#CDD6F4").unwrap().into();
    let directory = resolve(IconMode::NerdFont, "Source", EntryIconKind::Directory);
    let file = resolve(IconMode::NerdFont, "main.rs", EntryIconKind::File);

    assert_eq!(
        directory.display_color(
            EntryIconKind::Directory,
            directory_color,
            unknown_file_color,
            fallback
        ),
        directory_color
    );
    assert_eq!(
        directory.label_color(EntryIconKind::Directory, directory_color, fallback, false),
        directory_color
    );
    assert_eq!(
        file.label_color(EntryIconKind::File, directory_color, fallback, false),
        fallback
    );

    let unknown = resolve(IconMode::NerdFont, "unknown.wolf", EntryIconKind::File);
    assert_eq!(
        unknown.display_color(
            EntryIconKind::File,
            directory_color,
            unknown_file_color,
            fallback
        ),
        unknown_file_color
    );
}

#[test]
fn terminal_palette_theme_does_not_use_fixed_icon_rgb() {
    let file = resolve(IconMode::NerdFont, "main.rs", EntryIconKind::File);
    assert!(file.color.is_some());
    assert_eq!(
        file.display_color(
            EntryIconKind::File,
            Color::Indexed(12),
            Color::Indexed(8),
            Color::Reset
        ),
        Color::Indexed(8)
    );
    assert_eq!(
        file.label_color(EntryIconKind::File, Color::Indexed(12), Color::Reset, true),
        Color::Reset
    );
}

#[test]
fn terminal_theme_colors_names_from_devicons_without_reported_palette() {
    use crate::terminal_appearance::{Scheme, TerminalAppearance};

    for scheme in [Scheme::Dark, Scheme::Light] {
        let theme = crate::theme::derive_default_theme(TerminalAppearance {
            scheme,
            background: None,
            palette: None,
        });
        assert_eq!(theme.text, Color::Reset);
        for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::None] {
            let icon = resolve_for_theme(mode, "main.rs", EntryIconKind::File, &theme);
            assert_eq!(
                icon.file_label_color("main.rs", EntryIconKind::File, &theme, false),
                Color::Reset
            );
            assert_eq!(
                icon.file_label_color("main.rs", EntryIconKind::File, &theme, true),
                Color::from(icon.color.unwrap())
            );
        }
    }
}
