use super::*;

#[test]
fn icon_variant_uses_palette_background_and_theme_definition() {
    use crate::terminal_appearance::Scheme;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.update_terminal_appearance(TerminalAppearance {
        scheme: Scheme::Dark,
        background: Some(Rgb::parse("#000000").unwrap()),
        palette: Some(crate::theme::FAR_PALETTE),
    });
    runtime.use_theme("catppuccin_mocha").unwrap();
    runtime
        .lua
        .load("wolf.theme.base = '#ffffff'")
        .exec()
        .unwrap();
    assert!(runtime.theme().unwrap().light_file_icons);
    runtime
        .lua
        .load("wolf.theme.icon_variant = 'dark'")
        .exec()
        .unwrap();
    assert!(!runtime.theme().unwrap().light_file_icons);
    runtime
        .lua
        .load("wolf.theme.icon_variant = 'auto'")
        .exec()
        .unwrap();
    assert!(runtime.theme().unwrap().light_file_icons);
    runtime.lua.load("wolf.theme.define('my_mocha', { extends = 'catppuccin_mocha', icon_variant = 'dark' })").exec().unwrap();
    runtime.use_theme("my_mocha").unwrap();
    assert!(!runtime.theme().unwrap().light_file_icons);
    runtime
        .lua
        .load("wolf.theme.define('far_light', { extends = 'far', icon_variant = 'light' })")
        .exec()
        .unwrap();
    runtime.use_theme("far_light").unwrap();
    let far = runtime.theme().unwrap();
    assert!(far.light_file_icons);
    let file = crate::icons::resolve_for_theme(
        IconMode::NerdFont,
        "main.rs",
        crate::icons::EntryIconKind::File,
        &far,
    );
    assert!(far.file_color_matches.contains_key(&file.color.unwrap()));
    assert!(
        runtime
            .lua
            .load("wolf.theme.icon_variant = 'blue'")
            .exec()
            .is_err()
    );
}

#[test]
fn dynamic_file_color_policy_is_session_only_and_rejects_fixed_palettes() {
    use crate::theme::{DynamicFileColorPolicy, FileColorPolicy};
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("terminal").unwrap();
    runtime.update_terminal_appearance(TerminalAppearance {
        scheme: crate::terminal_appearance::Scheme::Dark,
        background: Some(Rgb::parse("#000000").unwrap()),
        palette: Some(crate::theme::FAR_PALETTE),
    });
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::OriginalDeviconsRgb
    );
    runtime
        .set_file_color_policy(DynamicFileColorPolicy::Nearest)
        .unwrap();
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::NearestPalette
    );
    runtime
        .set_file_color_policy(DynamicFileColorPolicy::Original)
        .unwrap();
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::OriginalDeviconsRgb
    );
    runtime
        .set_file_color_policy(DynamicFileColorPolicy::Nearest)
        .unwrap();
    runtime.use_theme("far").unwrap();
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::NearestPalette
    );
    assert!(
        runtime
            .set_file_color_policy(DynamicFileColorPolicy::Original)
            .is_err()
    );
    runtime.use_theme("terminal").unwrap();
    runtime
        .set_file_color_policy(DynamicFileColorPolicy::Nearest)
        .unwrap();
    runtime.use_palette("far").unwrap();
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        FileColorPolicy::NearestPalette
    );
}

#[test]
fn generated_terminal_theme_follows_terminal_appearance() {
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert_eq!(
        runtime.draft.borrow().active_theme.as_deref(),
        Some("catppuccin_mocha")
    );
    assert!(!runtime.draft.borrow().themes.contains_key("terminal"));
    runtime
        .lua
        .load("wolf.theme.use('terminal')")
        .exec()
        .unwrap();
    let syntax = runtime.syntax_theme().unwrap();

    let theme = runtime.theme().unwrap();
    assert_eq!(theme.base, Color::Reset);
    assert_eq!(theme.text, Color::Reset);
    assert_eq!(theme.active_border, Color::Indexed(4));
    assert_eq!(theme.selected, Color::Rgb(0, 45, 115));
    assert!(runtime.update_terminal_appearance(TerminalAppearance {
        scheme: crate::terminal_appearance::Scheme::Light,
        background: Some(Rgb {
            red: 255,
            green: 255,
            blue: 255
        }),
        palette: None,
    }));
    assert_eq!(runtime.theme().unwrap().selected, Color::Rgb(191, 216, 255));
    assert!(!runtime.update_terminal_appearance(TerminalAppearance {
        scheme: crate::terminal_appearance::Scheme::Light,
        background: Some(Rgb {
            red: 255,
            green: 255,
            blue: 255
        }),
        palette: None,
    }));
    assert_eq!(runtime.syntax_theme().unwrap(), syntax);
}

#[test]
fn terminal_file_colors_default_to_original_and_nearest_tracks_palette_updates() {
    use crate::icons::{EntryIconKind, resolve_for_theme};
    use crate::terminal_appearance::Scheme;
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("terminal").unwrap();
    let target = crate::icons::resolve(IconMode::NerdFont, "main.rs", EntryIconKind::File)
        .color
        .unwrap();
    for index in [3, 5] {
        let mut palette = [Rgb {
            red: 0,
            green: 0,
            blue: 0,
        }; 16];
        palette[index] = target;
        runtime.update_terminal_appearance(TerminalAppearance {
            scheme: Scheme::Dark,
            background: Some(Rgb {
                red: 0,
                green: 0,
                blue: 0,
            }),
            palette: Some(palette),
        });
        let theme = runtime.theme().unwrap();
        let icon = resolve_for_theme(IconMode::NerdFont, "main.rs", EntryIconKind::File, &theme);
        assert_eq!(
            theme.file_color_policy,
            crate::theme::FileColorPolicy::OriginalDeviconsRgb
        );
        assert_eq!(
            icon.themed_display_color(EntryIconKind::File, &theme),
            Color::from(target)
        );
        runtime
            .set_file_color_policy(crate::theme::DynamicFileColorPolicy::Nearest)
            .unwrap();
        let theme = runtime.theme().unwrap();
        assert_eq!(
            icon.themed_display_color(EntryIconKind::File, &theme),
            Color::Indexed(index as u8)
        );
        assert_eq!(
            icon.file_label_color("main.rs", EntryIconKind::File, &theme, true),
            Color::Indexed(index as u8)
        );
        runtime
            .set_file_color_policy(crate::theme::DynamicFileColorPolicy::Original)
            .unwrap();
    }
}

#[test]
fn user_theme_named_default_is_independent_of_terminal_and_appearance_option_is_validated() {
    use ratatui::style::Color;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime
        .lua
        .load("wolf.theme.define('default', { extends = 'catppuccin_mocha' })")
        .exec()
        .unwrap();
    runtime.use_theme("default").unwrap();
    assert_eq!(runtime.theme().unwrap().base, Color::Rgb(0x1e, 0x1e, 0x2e));
    runtime
        .lua
        .load("wolf.option.set('terminal.appearance', 'light')")
        .exec()
        .unwrap();
    assert_eq!(runtime.theme().unwrap().base, Color::Rgb(0x1e, 0x1e, 0x2e));
    assert!(
        runtime
            .lua
            .load("wolf.option.set('terminal.appearance', 'orange')")
            .exec()
            .is_err()
    );
    runtime.use_theme("terminal").unwrap();
    assert_eq!(runtime.theme().unwrap().selected, Color::Rgb(191, 216, 255));
    runtime.update_terminal_appearance(TerminalAppearance {
        scheme: crate::terminal_appearance::Scheme::Dark,
        background: Some(Rgb {
            red: 0,
            green: 0,
            blue: 0,
        }),
        palette: None,
    });
    assert_eq!(runtime.theme().unwrap().selected, Color::Rgb(191, 216, 255));
    runtime
        .lua
        .load("wolf.syntax.text = '#010203'")
        .exec()
        .unwrap();
    assert_eq!(
        runtime
            .syntax_theme()
            .unwrap()
            .color(crate::syntax::SyntaxCategory::Text, Color::Reset),
        Color::Rgb(1, 2, 3)
    );
}
