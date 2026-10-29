use super::super::appearance::validate_presets;
use super::*;

#[test]
fn terminal_style_override_enables_colored_file_names() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(!runtime.workspace_options().color_file_names);
    runtime
        .lua
        .load("wolf.style.color_file_names = true")
        .exec()
        .unwrap();
    assert!(runtime.workspace_options().color_file_names);
    assert!(runtime.display_style().unwrap().color_file_names);
}

#[test]
fn theme_facade_selects_syntax_and_preserves_independent_style() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime
        .lua
        .load(
            r##"
            local theme = require("commander_wolf").theme
            local syntax = wolf.syntax
            local style = wolf.style
            theme.use("far")
            assert(theme.base == "#000080")
            assert(style.icons == "nerd_font")
            local far_keyword = syntax.keyword
            style.use("classic")
            assert(style.icons == "none")
            theme.base = "#101014"
            syntax.keyword = "#123456"
            style.color_file_names = false
            assert(theme.base == "#101014")
            assert(syntax.keyword == "#123456")
            theme.use("catppuccin_mocha")
            assert(theme.base == "#1e1e2e")
            assert(syntax.keyword ~= far_keyword)
            assert(style.icons == "none")
            assert(style.color_file_names == false)
            syntax.use("far")
            assert(syntax.keyword == far_keyword)
            syntax.keyword = "#654321"
            theme.use("terminal")
            assert(theme.base == "default")
            assert(syntax.keyword ~= "#654321")
            assert(style.icons == "none")
            assert(style.color_file_names == false)
            theme.base = "#101014"
            style.color_file_names = true
            assert(theme.base == "#101014")
            assert(style.color_file_names == true)
        "##,
        )
        .exec()
        .unwrap();
    assert!(runtime.workspace_options().color_file_names);
    assert_eq!(
        runtime.theme().unwrap().base,
        ratatui::style::Color::Rgb(0x10, 0x10, 0x14)
    );
}

#[test]
fn theme_facade_defines_named_and_inline_syntax_with_extends() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime
        .lua
        .load(
            r##"
            local theme = wolf.theme
            wolf.syntax.define("my_syntax", { extends = "far", keyword = "#123456" })
            wolf.style.define("my_style", { extends = "modern", color_file_names = true })
            theme.define("custom", { extends = "catppuccin_mocha", base = "#101014", syntax = "my_syntax" })
            theme.use("custom")
            assert(theme.base == "#101014")
            assert(wolf.syntax.keyword == "#123456")
            assert(wolf.style.color_file_names == false)
            wolf.style.use("my_style")
            assert(wolf.style.color_file_names == true)
            theme.define("inline", {
                extends = "catppuccin_mocha",
                base = "#202025",
                syntax = { extends = "far", keyword = "#654321" },
            })
            theme.use("inline")
            assert(theme.base == "#202025")
            assert(wolf.syntax.keyword == "#654321")
            assert(wolf.style.color_file_names == true)
            theme.use("custom")
            assert(theme.base == "#101014")
            assert(wolf.syntax.keyword == "#123456")
        "##,
        )
        .exec()
        .unwrap();
}

#[test]
fn theme_facade_reports_unknown_presets_and_properties() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for (source, expected) in [
        (
            "wolf.theme.use('missing')",
            "unknown theme preset 'missing'",
        ),
        (
            "wolf.syntax.use('missing')",
            "unknown syntax preset 'missing'",
        ),
        (
            "wolf.style.use('missing')",
            "unknown style preset 'missing'",
        ),
        (
            "wolf.style.colro_file_names = true",
            "unknown style property 'colro_file_names'",
        ),
        ("wolf.theme.txe = '#ffffff'", "unknown theme property 'txe'"),
        (
            "wolf.syntax.kewyord = '#ffffff'",
            "unknown syntax property 'kewyord'",
        ),
        (
            "wolf.theme.palette = 'far'",
            "unknown theme property 'palette'",
        ),
    ] {
        let error = runtime.lua.load(source).exec().unwrap_err().to_string();
        assert!(error.contains(expected), "{source}: {error}");
    }
    assert_eq!(
        runtime.theme().unwrap().base,
        ratatui::style::Color::Rgb(0x1e, 0x1e, 0x2e)
    );
}

#[test]
fn user_lua_module_can_derive_and_override_a_shipped_theme() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(paths.config.join("themes")).unwrap();
    fs::write(
        paths.config.join("themes/mine.lua"),
        r#"
        wolf.style.define("my_modern", {
          extends = "modern",
          panels = { border = "double" },
        })
        wolf.theme.define("my_modern_theme", {
          extends = "catppuccin_mocha",
        })
    "#,
    )
    .unwrap();
    fs::write(
        paths.config.join("init.lua"),
        r#"
        require("themes.mine")
        wolf.theme.use("my_modern_theme")
        wolf.style.use("my_modern")
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    assert!(runtime.load_warning.is_none());
    assert_eq!(runtime.theme().unwrap().panel_border_set().top_left, "╔");
    assert_eq!(runtime.theme().unwrap().footer_border_set().top_left, "╭");
    runtime.use_theme("far").unwrap();
    assert_eq!(runtime.workspace_options().panel_style, PanelStyle::Framed);
    assert_eq!(runtime.theme().unwrap().panel_border_set().top_left, "╔");
}

#[test]
fn component_registry_rejects_reserved_names_and_inheritance_cycles() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .lua
            .load("wolf.syntax.define('off', { extends = 'terminal' })")
            .exec()
            .is_err()
    );
    runtime
        .lua
        .load(
            "wolf.style.define('first', { extends = 'second' }); \
             wolf.style.define('second', { extends = 'first' })",
        )
        .exec()
        .unwrap();
    assert!(
        validate_presets(&runtime.draft.borrow())
            .unwrap_err()
            .contains("style inheritance cycle")
    );
}

#[test]
fn generated_syntax_palette_can_be_replaced_independently_of_the_ui_theme() {
    use crate::syntax::SyntaxCategory;
    use crate::theme::Rgb;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("terminal").unwrap();
    let ui = runtime.theme().unwrap();
    assert_eq!(
        runtime
            .syntax_theme()
            .unwrap()
            .color(SyntaxCategory::Keyword, ui.text),
        ratatui::style::Color::Indexed(5)
    );
    runtime
        .lua
        .load(
            r##"
        local wolf = require('commander_wolf')
        wolf.syntax.define('custom', { text = '#010203', keyword = '#040506' })
        wolf.syntax.use('custom')
    "##,
        )
        .exec()
        .unwrap();
    assert_eq!(runtime.theme().unwrap(), ui);
    let syntax = runtime.syntax_theme().unwrap();
    assert_eq!(
        syntax.color(SyntaxCategory::Keyword, ui.text),
        Rgb::parse("#040506").unwrap().into()
    );
    assert_eq!(
        syntax.color(SyntaxCategory::String, ui.text),
        Rgb::parse("#010203").unwrap().into()
    );
    assert!(
        runtime
            .lua
            .load("wolf.syntax.use('missing')")
            .exec()
            .is_err()
    );
    assert!(
        runtime
            .lua
            .load("wolf.syntax.define('bad', { text = 'not-a-color' })")
            .exec()
            .is_err()
    );
    assert!(
        runtime
            .lua
            .load("wolf.syntax.define('bad', { text = '#000000', typo = '#ffffff' })")
            .exec()
            .is_err()
    );
    assert_eq!(runtime.syntax_theme().unwrap(), syntax);
}

#[test]
fn user_defined_theme_accepts_named_terminal_colors() {
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    let fields = runtime.lua.create_table().unwrap();
    for (name, value) in &runtime.draft.borrow().themes["catppuccin_mocha"] {
        fields.set(name.as_str(), value.as_str()).unwrap();
    }
    fields.set("active_border", "brightBlue").unwrap();
    fields.set("syntax", "terminal").unwrap();
    fields
        .set("visual_selection_background", "brightPurple")
        .unwrap();
    runtime
        .lua
        .globals()
        .set("user_theme_fields", fields)
        .unwrap();
    runtime
        .lua
        .load(
            "wolf.theme.define('user_named', user_theme_fields); \
             wolf.theme.use('user_named')",
        )
        .exec()
        .unwrap();

    let theme = runtime.theme().unwrap();
    assert_eq!(theme.active_border, Color::Indexed(12));
    assert_eq!(theme.visual_selection_background, Color::Indexed(13));
}

#[test]
fn icon_and_filename_color_properties_are_independent_and_validated() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert_eq!(runtime.workspace_options().icon_mode, IconMode::NerdFont);
    assert!(!runtime.workspace_options().color_file_names);
    for (name, mode) in [
        ("nerd_font", IconMode::NerdFont),
        ("unicode", IconMode::Unicode),
        ("none", IconMode::None),
    ] {
        for enabled in [false, true] {
            runtime
                .lua
                .load(format!(
                    "wolf.style.icons = '{name}'; wolf.style.color_file_names = {enabled}"
                ))
                .exec()
                .unwrap();
            assert_eq!(runtime.workspace_options().icon_mode, mode);
            assert_eq!(runtime.workspace_options().color_file_names, enabled);
        }
    }
    assert!(
        runtime
            .lua
            .load("wolf.style.color_file_names = 'yes'")
            .exec()
            .is_err()
    );
    assert!(
        runtime
            .lua
            .load("wolf.style.icons = 'emoji'")
            .exec()
            .is_err()
    );
}

#[test]
fn icon_and_frame_styles_are_independent_in_user_configuration() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.style.icons = 'none'\nwolf.style.frames.border = 'double'\nwolf.style.panels.border = 'double'\nwolf.style.footer.border = 'double'\nwolf.style.separator = 'double'",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    assert_eq!(runtime.workspace_options().icon_mode, IconMode::None);
    assert_eq!(runtime.theme().unwrap().border_set().top_left, "╔");
    assert_eq!(runtime.theme().unwrap().separator_style.horizontal(), "═");
    runtime.use_theme("catppuccin_mocha").unwrap();
    assert_eq!(runtime.theme().unwrap().border_set().top_left, "╔");

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.style.frames.border = 'missing'",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("missing"))
    );
}
