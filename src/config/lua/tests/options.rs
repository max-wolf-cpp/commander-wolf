use super::*;

#[test]
fn session_memory_is_opt_in_and_clean_ignores_the_user_setting() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    assert!(
        !ConfigRuntime::load(user_paths.clone(), false)
            .unwrap()
            .remember_directories()
    );
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.option.set('session.remember_directories', true)",
    )
    .unwrap();
    assert!(
        ConfigRuntime::load(user_paths.clone(), false)
            .unwrap()
            .remember_directories()
    );
    assert!(
        !ConfigRuntime::load(user_paths.clone(), true)
            .unwrap()
            .remember_directories()
    );
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.option.set('session.remember_directories', 'yes')",
    )
    .unwrap();
    let fallback = ConfigRuntime::load(user_paths, false).unwrap();
    assert!(!fallback.remember_directories());
    assert!(
        fallback
            .load_warning
            .unwrap()
            .contains("session.remember_directories must be a boolean")
    );
}

#[test]
fn shipped_workspace_options_and_user_overrides_are_typed() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        r#"
        wolf.option.set("workspace.line_numbers", "absolute")
        wolf.option.set("workspace.scroll_offset", 4)
        wolf.style.icons = "ascii"
        wolf.option.set("workspace.show_hidden", false)
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    assert_eq!(
        runtime.workspace_options(),
        WorkspaceOptions {
            line_numbers: LineNumberMode::Absolute,
            color_file_names: false,
            show_hidden: false,
            scroll_offset: 4,
            icon_mode: IconMode::None,
            panel_style: PanelStyle::Framed,
        }
    );
}

#[test]
fn status_mode_position_is_typed_and_invalid_values_roll_back() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.option.set('status.mode_position', 'right')\nwolf.style.footer.layout = 'framed'\nwolf.style.panels.layout = 'borderless'",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    assert_eq!(
        runtime.status_line_options(),
        StatusLineOptions {
            mode_position: ModePosition::Right,
            style: UiStyle::Framed,
        }
    );
    assert_eq!(
        runtime.workspace_options().panel_style,
        PanelStyle::Borderless
    );

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.option.set('status.mode_position', 'center')",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("status.mode_position"))
    );
    assert_eq!(runtime.status_line_options(), StatusLineOptions::default());

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.style.panels.layout = 'cornered'",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("panel style must be"))
    );
    assert_eq!(runtime.status_line_options(), StatusLineOptions::default());
}

#[test]
fn dialog_display_defaults_to_float_and_accepts_command_line() {
    let defaults = TestDirectory::new();
    let runtime = ConfigRuntime::load(paths(&defaults.0), true).unwrap();
    assert_eq!(runtime.dialog_options().display, DialogDisplay::Float);

    let configured = TestDirectory::new();
    let configured_paths = paths(&configured.0);
    fs::create_dir_all(&configured_paths.config).unwrap();
    fs::write(
        configured_paths.config.join("init.lua"),
        "wolf.option.set('ui.dialogs', 'command_line')",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(configured_paths, false).unwrap();
    assert_eq!(runtime.dialog_options().display, DialogDisplay::CommandLine);

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.option.set('ui.dialogs', 'popup')",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("ui.dialogs"))
    );
    assert_eq!(runtime.dialog_options().display, DialogDisplay::Float);
}

#[test]
fn invalid_workspace_option_rolls_back_to_shipped_defaults() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        "wolf.option.set('workspace.scroll_offset', -1)",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    assert!(runtime.load_warning.is_some());
    assert_eq!(
        runtime.workspace_options(),
        WorkspaceOptions {
            line_numbers: LineNumberMode::Relative,
            color_file_names: false,
            show_hidden: true,
            scroll_offset: 10,
            icon_mode: IconMode::NerdFont,
            panel_style: PanelStyle::Framed,
        }
    );
}

#[test]
fn workspace_option_errors_name_the_invalid_parameter() {
    let root = TestDirectory::new();
    let first_paths = paths(&root.0);
    fs::create_dir_all(&first_paths.config).unwrap();
    fs::write(
        first_paths.config.join("init.lua"),
        "wolf.option.set('workspace.line_numbers', 42)",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(first_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| { warning.contains("workspace.line_numbers") })
    );

    let second = TestDirectory::new();
    let second_paths = paths(&second.0);
    fs::create_dir_all(&second_paths.config).unwrap();
    fs::write(
        second_paths.config.join("init.lua"),
        "wolf.option.set('workspace.unknown', true)",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(second_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("workspace.unknown"))
    );

    let third = TestDirectory::new();
    let third_paths = paths(&third.0);
    fs::create_dir_all(&third_paths.config).unwrap();
    fs::write(
        third_paths.config.join("init.lua"),
        "wolf.style.icons = 'emoji'",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(third_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("emoji"))
    );
}

#[test]
fn finder_options_are_typed_overridable_and_bounded() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        r#"
        wolf.option.set("finder.show_hidden", true)
        wolf.option.set("finder.scroll_offset", 3)
        wolf.option.set("finder.max_results", 77)
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    assert_eq!(
        runtime.finder_options(),
        FinderOptions {
            show_hidden: true,
            scroll_offset: 3,
            max_results: 77,
        }
    );

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.option.set('finder.max_results', 0)",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| { warning.contains("finder.max_results") })
    );
    assert_eq!(runtime.finder_options(), FinderOptions::default());
}
