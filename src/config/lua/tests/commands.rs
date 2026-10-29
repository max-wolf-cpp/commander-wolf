use super::*;

#[test]
fn shipped_archive_aliases_dispatch_and_complete() {
    use crate::app::Action;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for (command, action) in [
        (":archive", Action::CreateArchive(None)),
        (
            ":archive \"new archive.zip\"",
            Action::CreateArchive(Some("new archive.zip".into())),
        ),
        (
            ":extract",
            Action::ExtractArchive {
                destination: None,
                other_panel: false,
            },
        ),
        (
            ":extract \"new folder\"",
            Action::ExtractArchive {
                destination: Some("new folder".into()),
                other_panel: false,
            },
        ),
        (
            ":extractother",
            Action::ExtractArchive {
                destination: None,
                other_panel: true,
            },
        ),
    ] {
        assert_eq!(
            runtime.command_line(command).unwrap(),
            CommandDispatch::Actions(vec![action])
        );
    }
    for (prefix, expected) in [
        (":arch", "archive"),
        (":extr", "extract"),
        (":extr", "extractother"),
    ] {
        assert!(
            runtime
                .command_completions(prefix)
                .iter()
                .any(|item| item.name == expected)
        );
    }
}

#[test]
fn shipped_clipboard_aliases_dispatch_and_complete() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for (command, action) in [
        (":copypath", crate::app::Action::CopySelectedPath),
        (":copyname", crate::app::Action::CopySelectedName),
    ] {
        assert_eq!(
            runtime.command_line(command).unwrap(),
            CommandDispatch::Actions(vec![action])
        );
        assert!(runtime.command_line(&format!("{command} extra")).is_err());
    }

    let completions = runtime.command_completions(":copy");
    for name in ["copy", "copyname", "copypath"] {
        assert!(completions.iter().any(|item| item.name == name));
    }
}

#[test]
fn component_commands_switch_only_their_layer_and_complete_live_names() {
    use crate::app::Action;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert_eq!(
        runtime.command_line(":layout compact").unwrap(),
        CommandDispatch::Actions(vec![Action::SetStylePreset(UiPreset::Compact)])
    );
    assert_eq!(
        runtime.command_line(":border double").unwrap(),
        CommandDispatch::Actions(vec![Action::SetBorderStyle(
            BorderStyle::parse("double").unwrap()
        )])
    );
    assert_eq!(
        runtime.command_line(":separator off").unwrap(),
        CommandDispatch::Actions(vec![Action::SetSeparatorStyle(SeparatorStyle::Off)])
    );
    assert_eq!(
        runtime.command_line(":colorscheme far").unwrap(),
        CommandDispatch::Actions(vec![Action::UsePalette("far".into())])
    );
    assert_eq!(
        runtime.command_line(":syntax off").unwrap(),
        CommandDispatch::Actions(vec![Action::UseSyntax("off".into())])
    );
    assert_eq!(
        runtime.command_line(":style classic").unwrap(),
        CommandDispatch::Actions(vec![Action::UseDisplayStyle("classic".into())])
    );
    assert_eq!(
        runtime
            .command_completions(":style ")
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["classic", "modern", "nvim"]
    );
    assert_eq!(
        runtime
            .command_completions(":syntax ")
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        [
            "catppuccin_frappe",
            "catppuccin_latte",
            "catppuccin_macchiato",
            "catppuccin_mocha",
            "dracula",
            "far",
            "gruvbox_dark",
            "gruvbox_light",
            "nord",
            "off",
            "one_dark",
            "solarized_dark",
            "solarized_light",
            "terminal",
            "tokyonight_night",
        ]
    );
    assert!(runtime.command_line(":border light_double_dashed").is_err());
    runtime.use_theme("far").unwrap();
    runtime.use_palette("catppuccin_mocha").unwrap();
    runtime.use_syntax("off").unwrap();
    runtime.use_style("nvim").unwrap();
    assert!(!runtime.syntax_enabled());
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        crate::theme::FileColorPolicy::DeviconsRgb
    );
    assert_eq!(
        runtime.display_style().unwrap().panels,
        PanelStyle::Borderless
    );
    runtime.use_theme("far").unwrap();
    assert!(runtime.syntax_enabled());
    assert_eq!(
        runtime.theme().unwrap().file_color_policy,
        crate::theme::FileColorPolicy::NearestPalette
    );
    assert_eq!(
        runtime.display_style().unwrap().panels,
        PanelStyle::Borderless
    );
}

#[test]
fn theme_command_switches_session_theme_and_rejects_unknown_names() {
    use ratatui::style::Color;

    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(runtime.use_theme("default").is_err());
    assert!(runtime.use_palette("default").is_err());
    assert!(runtime.use_syntax("default").is_err());
    assert!(runtime.use_style("default").is_err());
    let names = runtime
        .command_completions(":theme ")
        .into_iter()
        .map(|item| item.name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"terminal".to_owned()));
    assert!(!names.contains(&"default".to_owned()));
    runtime.use_theme("catppuccin_mocha").unwrap();
    let mocha = runtime.theme().unwrap();
    let syntax = runtime.syntax_theme().unwrap();
    let CommandDispatch::Actions(actions) = runtime.command_line(":theme terminal").unwrap() else {
        panic!("theme command should dispatch a built-in action");
    };
    assert_eq!(
        actions,
        vec![crate::app::Action::UseTheme("terminal".into())]
    );
    runtime.use_theme("terminal").unwrap();
    assert_eq!(runtime.theme().unwrap().base, Color::Reset);
    assert_ne!(runtime.syntax_theme().unwrap(), syntax);
    assert_eq!(
        runtime.syntax_theme().unwrap(),
        SyntaxTheme::generated_default()
    );

    assert!(runtime.use_theme("default").is_err());
    runtime.use_theme("terminal").unwrap();
    assert_eq!(
        runtime.theme().unwrap().selected_text,
        Color::Rgb(255, 255, 255)
    );
    assert!(runtime.use_theme("missing").is_err());
    assert_eq!(runtime.theme().unwrap().base, Color::Reset);
    runtime.use_theme("catppuccin_mocha").unwrap();
    assert_eq!(runtime.theme().unwrap(), mocha);
}

#[test]
fn lua_command_accepts_an_argument_array() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        r#"wolf.api.command("file.rename", { "新 name.txt" })"#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    assert!(matches!(
        runtime.take_requests().as_slice(),
        [LuaRequest::Execute { command: CommandId(id), args, .. }]
            if id == "file.rename" && args == &vec!["新 name.txt".to_owned()]
    ));
}

#[test]
fn custom_command_can_notify_through_typed_request() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        r#"
        wolf.command.create("project_root", function(context)
          wolf.api.notify("count=" .. tostring(context.count or "none"))
        end, { description = "test command" })
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    let CommandDispatch::Lua {
        callback,
        invocation,
    } = runtime.command_line(":project_root").unwrap()
    else {
        panic!("Lua command expected")
    };
    let suggestions = runtime.command_completions(":proj");
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].name, "project_root");
    assert_eq!(suggestions[0].description, "test command");
    assert_eq!(
        runtime
            .call_lua(callback, invocation.arguments, invocation.count)
            .unwrap(),
        vec![LuaRequest::Notify("count=none".into())]
    );
}
