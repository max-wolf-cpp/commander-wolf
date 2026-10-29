use super::borders::parse_border_type;
use super::*;
use crate::terminal_appearance::{Scheme, TerminalAppearance};

#[test]
fn normal_badge_background_override_preserves_other_mode_backgrounds() {
    let mut theme = Theme::emergency_monochrome();
    theme.compact_mode_background = Some(Color::Blue);
    theme
        .apply_palette_fields(&HashMap::from([(
            "compact_mode_normal_background".into(),
            "#008080".into(),
        )]))
        .unwrap();

    assert_eq!(
        theme.compact_mode_background_color(StatusMode::Normal),
        Color::Rgb(0, 128, 128)
    );
    for mode in [
        StatusMode::Insert,
        StatusMode::Search,
        StatusMode::Visual,
        StatusMode::Command,
    ] {
        assert_eq!(theme.compact_mode_background_color(mode), Color::Blue);
    }
    theme.compact_mode_background = None;
    theme.mode_command = Color::Yellow;
    assert_eq!(
        theme.compact_mode_background_color(StatusMode::Command),
        Color::Yellow
    );
}

#[test]
fn generated_selection_search_and_capsule_pairs_are_readable_on_dark_palettes_and_light_samples() {
    for (scheme, backgrounds) in [
        (
            Scheme::Dark,
            &[
                "#090300", "#161719", "#31363B", "#131313", "#000000", "#303446", "#24273A",
                "#1E1E2E", "#181818", "#1E1F29", "#1F1D27", "#1C1E26",
            ][..],
        ),
        (
            Scheme::Light,
            &["#FFFFFF", "#F7F7F7", "#E5E5E5", "#D0D0D0"][..],
        ),
    ] {
        for background in backgrounds {
            let theme = derive_default_theme(TerminalAppearance {
                scheme,
                background: Some(Rgb::parse(background).unwrap()),
                palette: None,
            });

            assert_eq!(theme.preview_title_text, Some(Color::Reset));
            assert_eq!(theme.borderless_preview_title_background, Color::Reset);
            assert!(contrast(theme.hidden, Rgb::parse(background).unwrap().into()) >= 3.0);
            for (text, surface) in [
                (theme.selected_text, theme.selected),
                (
                    theme.visual_selection_text,
                    theme.visual_selection_background,
                ),
                (theme.search_match_text, theme.search_match_background),
                (
                    theme.search_current_match_text,
                    theme.search_current_match_background,
                ),
                (
                    theme.inactive_selected_text.unwrap(),
                    theme.inactive_selected.unwrap(),
                ),
            ] {
                assert!(
                    contrast(text, surface) >= 4.5,
                    "{scheme:?} {background} {text:?} on {surface:?}"
                );
            }
            assert_eq!(
                theme.borderless_active_title_background,
                theme.active_border
            );
            assert_eq!(
                theme.borderless_inactive_title_background,
                theme.inactive_border
            );
            assert_eq!(theme.borderless_inactive_title_text, Some(Color::Reset));
        }
    }
}

#[test]
fn generated_selection_uses_terminal_blue_and_independent_title_colors() {
    let mut palette = FAR_PALETTE;
    for (scheme, blue) in [
        (Scheme::Dark, Rgb::parse("#000080").unwrap()),
        (Scheme::Light, Rgb::parse("#BBDDFF").unwrap()),
    ] {
        palette[4] = blue;
        let appearance = TerminalAppearance {
            scheme,
            background: Some(if scheme == Scheme::Light {
                Rgb::parse("#FFFFFF").unwrap()
            } else {
                Rgb::parse("#000000").unwrap()
            }),
            palette: Some(palette),
        };
        let mut theme = derive_default_theme(appearance);
        let fallback = derive_default_theme(TerminalAppearance {
            palette: None,
            ..appearance
        });

        assert_eq!(theme.selected, Color::Indexed(4));
        let Color::Indexed(text_index) = theme.selected_text else {
            panic!("selection text should use the reported terminal palette")
        };
        assert!(crate::palette_colors::contrast(palette[text_index as usize], blue) >= 4.5);
        assert_eq!(
            theme.borderless_active_title_background,
            theme.active_border
        );
        let Color::Indexed(title_text_index) = theme.borderless_title_text else {
            panic!("active title text should use the reported terminal palette")
        };
        assert!(
            crate::palette_colors::contrast(palette[title_text_index as usize], palette[4]) >= 4.5
        );
        assert_eq!(
            theme.borderless_inactive_title_background,
            theme.inactive_border
        );
        assert_eq!(theme.active_border, Color::Indexed(4));
        assert_eq!(theme.active_title, Color::Indexed(4));
        assert_eq!(theme.directory, Color::Indexed(4));
        assert_eq!(theme.current_line_number, Color::Indexed(4));
        assert_eq!(theme.line_number, theme.hidden);
        let Color::Rgb(red, green, blue) = theme.visual_selection_background else {
            panic!("visual range should use a neutral surface")
        };
        assert_eq!(red, green);
        assert_eq!(green, blue);
        for (text, surface) in [
            (
                theme.visual_selection_text,
                theme.visual_selection_background,
            ),
            (theme.search_match_text, theme.search_match_background),
            (
                theme.search_current_match_text,
                theme.search_current_match_background,
            ),
        ] {
            let (Color::Indexed(text_index), Color::Rgb(red, green, blue)) = (text, surface) else {
                panic!("highlight text should use the terminal palette")
            };
            assert!(
                crate::palette_colors::contrast(
                    palette[usize::from(text_index)],
                    Rgb { red, green, blue }
                ) >= 4.5
            );
        }

        let mut changed_palette = palette;
        changed_palette[3] = Rgb::parse("#FF0000").unwrap();
        changed_palette[6] = Rgb::parse("#00FF00").unwrap();
        let changed = derive_default_theme(TerminalAppearance {
            palette: Some(changed_palette),
            ..appearance
        });
        assert_ne!(
            theme.search_match_background,
            changed.search_match_background
        );
        assert_ne!(
            theme.search_current_match_background,
            changed.search_current_match_background
        );
        assert_ne!(
            theme.inactive_selected,
            Some(theme.borderless_inactive_title_background)
        );
        assert_eq!(theme.compact_mode_background, None);
        assert_eq!(fallback.compact_mode_background, Some(fallback.selected));
        assert_eq!(theme.compact_mode_text, fallback.compact_mode_text);
        for (mode, index) in [
            (StatusMode::Search, 6),
            (StatusMode::Insert, 2),
            (StatusMode::Command, 3),
            (StatusMode::Normal, 4),
            (StatusMode::Visual, 5),
        ] {
            assert_eq!(theme.mode_color(mode), Color::Indexed(index));
            assert_eq!(fallback.mode_color(mode), Color::Indexed(index));
            let Color::Indexed(text_index) = theme.compact_mode_text_color(mode) else {
                panic!("mode text should use the reported terminal palette")
            };
            assert!(
                crate::palette_colors::contrast(
                    palette[usize::from(text_index)],
                    palette[usize::from(index)]
                ) >= 4.5
            );
        }

        theme
            .apply_palette_fields(&HashMap::from([(
                "compact_mode_text".into(),
                "#010203".into(),
            )]))
            .unwrap();
        assert_eq!(
            theme.compact_mode_text_color(StatusMode::Normal),
            Color::Rgb(1, 2, 3)
        );

        theme
            .apply_palette_fields(&HashMap::from([("selected".into(), "#010203".into())]))
            .unwrap();
        assert_eq!(theme.borderless_active_title_background, Color::Indexed(4));
    }
}

#[test]
fn border_style_accepts_supported_names_and_rejects_unknown_values() {
    assert_eq!(BorderStyle::parse("simple").unwrap(), BorderStyle::Simple);
    assert_eq!(
        BorderStyle::parse("double").unwrap(),
        BorderStyle::Unicode(BorderType::Double)
    );
    assert_eq!(parse_border_type("plain").unwrap(), BorderType::Plain);
    assert_eq!(parse_border_type("rounded").unwrap(), BorderType::Rounded);
    assert_eq!(parse_border_type("double").unwrap(), BorderType::Double);
    assert!(parse_border_type("wolf").is_err());

    let mut theme = Theme::emergency_monochrome();
    theme.set_border_style(BorderStyle::Simple);
    assert_eq!(theme.border_set().top_left, "+");
    theme.set_border_style(BorderStyle::Unicode(BorderType::Double));
    assert_eq!(theme.border_set().top_left, "╔");
}

#[test]
fn theme_colors_accept_terminal_defaults_named_slots_and_ansi_numbers() {
    assert_eq!(parse_theme_color("default").unwrap(), Color::Reset);
    assert_eq!(parse_theme_color("ansi0").unwrap(), Color::Indexed(0));
    assert_eq!(parse_theme_color("ansi15").unwrap(), Color::Indexed(15));
    for (index, name) in [
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "purple",
        "cyan",
        "white",
        "brightBlack",
        "brightRed",
        "brightGreen",
        "brightYellow",
        "brightBlue",
        "brightPurple",
        "brightCyan",
        "brightWhite",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            parse_theme_color(name).unwrap(),
            Color::Indexed(index as u8)
        );
    }
    assert_eq!(parse_theme_color("magenta").unwrap(), Color::Indexed(5));
    assert_eq!(
        parse_theme_color("brightMagenta").unwrap(),
        Color::Indexed(13)
    );
    assert_eq!(
        parse_theme_color("#89B4FA").unwrap(),
        Color::Rgb(0x89, 0xB4, 0xFA)
    );
    assert!(parse_theme_color("ansi16").is_err());
    assert!(parse_theme_color("ansi-1").is_err());
    assert!(parse_theme_color("brightOrange").is_err());
    assert!(parse_theme_color("ééé").is_err());
}

#[test]
fn dialog_kinds_have_independent_theme_accents() {
    let mut theme = Theme::emergency_monochrome();
    theme.dialog_info = Rgb::parse("#89B4FA").unwrap().into();
    theme.dialog_warning = Rgb::parse("#FAB387").unwrap().into();
    theme.dialog_error = Rgb::parse("#F38BA8").unwrap().into();

    assert_eq!(theme.dialog_color(DialogKind::Info), theme.dialog_info);
    assert_eq!(
        theme.dialog_color(DialogKind::Warning),
        theme.dialog_warning
    );
    assert_eq!(theme.dialog_color(DialogKind::Error), theme.dialog_error);
    for kind in [DialogKind::Info, DialogKind::Warning, DialogKind::Error] {
        let colors = theme.dialog_colors(kind);
        assert_eq!(colors.background, theme.float_background);
        assert_eq!(colors.text, theme.text);
        assert_eq!(colors.accent, theme.dialog_color(kind));
        assert_eq!(colors.secondary, theme.inactive_title);
        assert_eq!(
            theme.dialog_shortcut_color(kind, colors.accent),
            colors.accent
        );
    }
}

fn contrast(foreground: Color, background: Color) -> f64 {
    let luminance = |color| {
        let Color::Rgb(red, green, blue) = color else {
            panic!("expected a derived RGB color")
        };

        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    };

    let first = luminance(foreground);
    let second = luminance(background);

    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}
