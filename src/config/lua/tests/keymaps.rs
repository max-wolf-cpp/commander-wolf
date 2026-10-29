use super::*;

#[test]
fn shipped_archive_leader_mappings_dispatch() {
    use crate::app::Action;
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for (context, suffix, expected) in [
        (InputContext::Normal, 'a', Action::CreateArchive(None)),
        (InputContext::Visual, 'a', Action::CreateArchive(None)),
        (
            InputContext::Normal,
            'x',
            Action::ExtractArchive {
                destination: None,
                other_panel: false,
            },
        ),
        (
            InputContext::Normal,
            'X',
            Action::ExtractArchive {
                destination: None,
                other_panel: true,
            },
        ),
    ] {
        let mut resolver = KeyResolver::default();
        for character in [' ', 'z'] {
            assert!(matches!(
                resolver.resolve(
                    &runtime.keymaps(),
                    context,
                    KeyChord {
                        code: KeyCode::Char(character),
                        modifiers: KeyModifiers::NONE
                    }
                ),
                ResolveOutcome::Pending(_)
            ));
        }
        let ResolveOutcome::Command(invocation) = resolver.resolve(
            &runtime.keymaps(),
            context,
            KeyChord {
                code: KeyCode::Char(suffix),
                modifiers: KeyModifiers::NONE,
            },
        ) else {
            panic!("archive mapping must resolve to a command");
        };
        assert_eq!(
            runtime
                .dispatch(&invocation.id, invocation.arguments, invocation.count)
                .unwrap(),
            CommandDispatch::Actions(vec![expected])
        );
    }
}

#[test]
fn shipped_clipboard_leader_mappings_dispatch_in_normal_and_visual_modes() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for context in [InputContext::Normal, InputContext::Visual] {
        for (suffix, action) in [
            ('p', crate::app::Action::CopySelectedPath),
            ('n', crate::app::Action::CopySelectedName),
        ] {
            let mut resolver = KeyResolver::default();
            for character in [' ', 'y'] {
                assert!(matches!(
                    resolver.resolve(
                        &runtime.keymaps(),
                        context,
                        KeyChord {
                            code: KeyCode::Char(character),
                            modifiers: KeyModifiers::NONE,
                        }
                    ),
                    ResolveOutcome::Pending(_)
                ));
            }

            let ResolveOutcome::Command(invocation) = resolver.resolve(
                &runtime.keymaps(),
                context,
                KeyChord {
                    code: KeyCode::Char(suffix),
                    modifiers: KeyModifiers::NONE,
                },
            ) else {
                panic!("clipboard mapping must resolve to a command");
            };
            assert_eq!(
                runtime
                    .dispatch(&invocation.id, invocation.arguments, invocation.count)
                    .unwrap(),
                CommandDispatch::Actions(vec![action])
            );
        }
    }
}

#[test]
fn shipped_mapping_and_theme_come_from_lua() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime.use_theme("catppuccin_mocha").unwrap();
    let mut resolver = KeyResolver::default();
    let outcome = resolver.resolve(
        &runtime.keymaps(),
        InputContext::Normal,
        KeyChord {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
        },
    );
    assert!(
        matches!(outcome, ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "selection.next")
    );
    assert_eq!(
        runtime.theme().unwrap().active_border,
        Rgb {
            red: 0x89,
            green: 0xB4,
            blue: 0xFA
        }
        .into()
    );
    let theme = runtime.theme().unwrap();
    assert_eq!(theme.line_number, Rgb::parse("#45475A").unwrap().into());
    assert_eq!(
        theme.current_line_number,
        Rgb::parse("#A6E3A1").unwrap().into()
    );
    assert_eq!(theme.selected, Rgb::parse("#313244").unwrap().into());
    assert_eq!(theme.selected_text, theme.text);
    assert_eq!(
        theme.visual_selection_background,
        Rgb::parse("#F9E2AF").unwrap().into()
    );
    assert_eq!(
        theme.visual_selection_text,
        Rgb::parse("#1E1E2E").unwrap().into()
    );
    assert_eq!(theme.directory, Rgb::parse("#89B4FA").unwrap().into());
    assert_eq!(
        theme.unknown_file_icon,
        Rgb::parse("#6C7086").unwrap().into()
    );
    assert_eq!(
        theme.framed_footer_border,
        Rgb::parse("#6C7086").unwrap().into()
    );
    assert_eq!(theme.dialog_info, Rgb::parse("#89B4FA").unwrap().into());
    assert_eq!(theme.dialog_warning, Rgb::parse("#FAB387").unwrap().into());
    assert_eq!(theme.dialog_error, Rgb::parse("#F38BA8").unwrap().into());
    assert!(theme.framed_footer_border_follows_mode);
    let mut legacy_fields = runtime
        .draft
        .borrow()
        .themes
        .get("catppuccin_mocha")
        .unwrap()
        .clone();
    for (name, actual) in [
        ("directory_edit_create", theme.directory_edit_create),
        ("directory_edit_rename", theme.directory_edit_rename),
        ("directory_edit_delete", theme.directory_edit_delete),
    ] {
        assert_eq!(actual, Rgb::parse(&legacy_fields[name]).unwrap().into());
    }
    legacy_fields.remove("framed_footer_border");
    legacy_fields.remove("framed_footer_border_follows_mode");
    legacy_fields.remove("dialog_info");
    legacy_fields.remove("dialog_warning");
    legacy_fields.remove("dialog_error");
    legacy_fields.remove("separator_style");
    legacy_fields.remove("directory_edit_create");
    legacy_fields.remove("directory_edit_rename");
    legacy_fields.remove("directory_edit_delete");
    legacy_fields.remove("visual_selection_background");
    legacy_fields.remove("visual_selection_text");
    let legacy_theme = Theme::from_fields(&legacy_fields).unwrap();
    assert_eq!(
        legacy_theme.visual_selection_background,
        legacy_theme.selected
    );
    assert_eq!(legacy_theme.visual_selection_text, legacy_theme.text);
    assert!(legacy_theme.framed_footer_border_follows_mode);
    assert_eq!(
        legacy_theme.framed_footer_border,
        legacy_theme.status_border
    );
    assert_eq!(legacy_theme.dialog_info, legacy_theme.active_border);
    assert_eq!(legacy_theme.dialog_warning, legacy_theme.mode_command);
    assert_eq!(legacy_theme.dialog_error, legacy_theme.error);
    assert_eq!(legacy_theme.directory_edit_create, legacy_theme.mode_insert);
    assert_eq!(
        legacy_theme.directory_edit_rename,
        legacy_theme.dialog_warning
    );
    assert_eq!(legacy_theme.directory_edit_delete, legacy_theme.error);
    legacy_fields.insert("directory_edit_create".into(), "#010203".into());
    legacy_fields.insert("directory_edit_rename".into(), "#040506".into());
    legacy_fields.insert("directory_edit_delete".into(), "#070809".into());
    let custom_theme = Theme::from_fields(&legacy_fields).unwrap();
    assert_eq!(
        custom_theme.directory_edit_create,
        Rgb::parse("#010203").unwrap().into()
    );
    assert_eq!(
        custom_theme.directory_edit_rename,
        Rgb::parse("#040506").unwrap().into()
    );
    assert_eq!(
        custom_theme.directory_edit_delete,
        Rgb::parse("#070809").unwrap().into()
    );
    assert_eq!(
        legacy_theme.separator_style,
        crate::theme::SeparatorStyle::Thin
    );
    for (name, expected) in [
        ("thin", crate::theme::SeparatorStyle::Thin),
        ("double", crate::theme::SeparatorStyle::Double),
        ("thick", crate::theme::SeparatorStyle::Thick),
        ("simple", crate::theme::SeparatorStyle::Simple),
    ] {
        legacy_fields.insert("separator_style".into(), name.into());
        assert_eq!(
            Theme::from_fields(&legacy_fields).unwrap().separator_style,
            expected
        );
    }
    legacy_fields.insert("border_style".into(), "simple".into());
    assert_eq!(
        Theme::from_fields(&legacy_fields)
            .unwrap()
            .border_set()
            .top_left,
        "+"
    );
    legacy_fields.insert("separator_style".into(), "heavy_double_dashed".into());
    assert!(
        Theme::from_fields(&legacy_fields)
            .unwrap_err()
            .contains("separator_style")
    );
    assert_eq!(theme.border_type, ratatui::widgets::BorderType::Rounded);
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "panel.open_selected"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "file.open_selected"
    ));
}

#[test]
fn shipped_file_operation_aliases_and_keymaps_are_lua_owned() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for alias in [
        "cd",
        "refresh",
        "reveal",
        "editor",
        "copy",
        "move",
        "delete",
        "rename",
        "mkdir",
        "touch",
        "style",
        "layout",
        "border",
        "separator",
        "colorscheme",
        "syntax",
        "icons",
        "name-colors",
        "hidden",
    ] {
        assert!(
            runtime
                .draft
                .borrow()
                .commands
                .resolve_alias(alias)
                .is_some(),
            "{alias}"
        );
    }
    let mut resolver = KeyResolver::default();
    for (code, modifiers, expected) in [
        (KeyCode::F(5), KeyModifiers::NONE, "file.copy_to_other"),
        (KeyCode::F(6), KeyModifiers::NONE, "file.move_to_other"),
        (KeyCode::F(8), KeyModifiers::SHIFT, "file.delete"),
    ] {
        let outcome = resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord { code, modifiers },
        );
        assert!(
            matches!(outcome, ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == expected)
        );
    }
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "file.toggle_mark"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "file.reveal_selected"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('o'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "editor.open_cwd"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "visual.begin"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('V'),
                modifiers: KeyModifiers::NONE
            }
        ),
        ResolveOutcome::Unmapped { .. }
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Visual,
            KeyChord {
                code: KeyCode::F(5),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
            if id == "file.copy_to_other"
    ));
    for (character, expected) in [
        ('o', "confirmation.overwrite"),
        ('s', "confirmation.skip"),
        (' ', "confirmation.toggle_all"),
    ] {
        assert!(matches!(
            resolver.resolve(
                &runtime.keymaps(),
                InputContext::Confirm,
                KeyChord {
                    code: KeyCode::Char(character),
                    modifiers: KeyModifiers::NONE,
                },
            ),
            ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. })
                if id == expected
        ));
    }
    assert!(matches!(
        runtime.command_line(":cd directory with spaces").unwrap(),
        CommandDispatch::Actions(actions)
            if actions == vec![crate::app::Action::ChangeDirectory("directory with spaces".into())]
    ));
    assert!(matches!(
        runtime.command_line(":reveal").unwrap(),
        CommandDispatch::Actions(actions)
            if actions == vec![crate::app::Action::RevealSelectedInFileManager]
    ));
    assert!(matches!(
        runtime.command_line(":editor code --new-window").unwrap(),
        CommandDispatch::Actions(actions)
            if actions == vec![crate::app::Action::OpenEditorInActiveDirectory(Some(
                Application::new("code".into(), vec!["--new-window".into()]).unwrap()
            ))]
    ));
}

#[test]
fn user_keymap_description_appears_in_help() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.keymap.set('normal', 'y', 'file.copy_to_other', { description = 'Copy selection short alias' })",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    let help = crate::help::HelpState::from_keymaps(&runtime.keymaps());
    assert!(help.entries.iter().any(|entry| {
        entry.context == InputContext::Normal
            && entry.keys == "y"
            && entry.description == "Copy selection short alias"
    }));
}

#[test]
fn shipped_keymaps_have_descriptions_visible_to_rust() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    for context in [
        InputContext::Normal,
        InputContext::Visual,
        InputContext::Insert,
        InputContext::Command,
        InputContext::Confirm,
        InputContext::Search,
        InputContext::FinderInsert,
        InputContext::FinderSearching,
        InputContext::FinderNormal,
        InputContext::Tree,
        InputContext::Viewer,
    ] {
        let maps = runtime.keymaps();
        let bindings = maps.bindings(context);
        assert!(!bindings.is_empty(), "{context:?}");
        assert!(
            bindings.iter().all(|binding| binding
                .description
                .as_ref()
                .is_some_and(|description| !description.trim().is_empty())),
            "{context:?}"
        );
    }
    assert_eq!(
        runtime.keymaps().bindings(InputContext::Normal)[0]
            .description
            .as_deref(),
        Some("Move selection down")
    );
}

#[test]
fn lua_keymap_description_is_validated_and_available_to_rust() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    runtime
        .lua
        .load("wolf.keymap.set('normal', 'j', 'app.quit', { description = 'Leave the app' })")
        .exec()
        .unwrap();
    let description = || {
        runtime
            .keymaps()
            .bindings(InputContext::Normal)
            .iter()
            .find(|binding| {
                binding.sequence
                    == [KeyChord {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::NONE,
                    }]
            })
            .and_then(|binding| binding.description.clone())
    };
    assert_eq!(description().as_deref(), Some("Leave the app"));
    assert!(
        runtime
            .lua
            .load("wolf.keymap.set('normal', 'j', 'app.quit', { description = 42 })")
            .exec()
            .is_err()
    );
    assert_eq!(description().as_deref(), Some("Leave the app"));
    runtime
        .lua
        .load("wolf.keymap.set('normal', 'j', 'app.quit')")
        .exec()
        .unwrap();
    assert_eq!(description(), None);
}

#[test]
fn user_can_replace_delete_and_define_leader_mapping() {
    let root = TestDirectory::new();
    let first_paths = paths(&root.0);
    fs::create_dir_all(&first_paths.config).unwrap();
    fs::write(
        first_paths.config.join("init.lua"),
        r#"
        wolf.keymap.del("normal", "j")
        wolf.g.mapleader = ","
        wolf.keymap.set("normal", "<Leader>x", "app.quit")
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(first_paths, false).unwrap();
    let mut resolver = KeyResolver::default();
    let plain = KeyChord {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
    };
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain),
        ResolveOutcome::Unmapped { .. }
    ));
    let comma = KeyChord {
        code: KeyCode::Char(','),
        modifiers: KeyModifiers::NONE,
    };
    let x = KeyChord {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::NONE,
    };
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, comma),
        ResolveOutcome::Pending(_)
    ));
    assert!(
        matches!(resolver.resolve(&runtime.keymaps(), InputContext::Normal, x), ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "app.quit")
    );
}

#[test]
fn user_can_clear_a_shipped_context() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(paths.config.join("init.lua"), "wolf.keymap.clear('normal')").unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    let mut resolver = KeyResolver::default();
    let q = KeyChord {
        code: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
    };
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, q),
        ResolveOutcome::Unmapped { .. }
    ));
}

#[test]
fn user_can_replace_file_operation_and_delete_confirmation_mappings() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        r#"
        wolf.keymap.del("normal", "<F5>")
        wolf.keymap.set("normal", "<F5>", "app.quit")
        wolf.keymap.del("confirm", "y")
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    let mut resolver = KeyResolver::default();
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord { code: KeyCode::F(5), modifiers: KeyModifiers::NONE },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "app.quit"
    ));
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Confirm,
            KeyChord {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE
            },
        ),
        ResolveOutcome::Unmapped { .. }
    ));
}

#[test]
fn lua_function_keymap_runs_through_the_command_queue() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        r#"wolf.keymap.set("normal", "x", function(context)
          wolf.api.notify("mapped=" .. tostring(context.count or "none"))
        end, { description = "Notify from Lua" })"#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths, false).unwrap();
    assert!(
        runtime
            .keymaps()
            .bindings(InputContext::Normal)
            .iter()
            .any(|binding| binding.description.as_deref() == Some("Notify from Lua"))
    );
    let mut resolver = KeyResolver::default();
    let x = KeyChord {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::NONE,
    };
    let ResolveOutcome::Command(invocation) =
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, x)
    else {
        panic!("Lua mapping expected")
    };
    let CommandDispatch::Lua {
        callback,
        invocation,
    } = runtime
        .dispatch(&invocation.id, invocation.arguments, invocation.count)
        .unwrap()
    else {
        panic!("Lua callback expected")
    };
    assert_eq!(
        runtime
            .call_lua(callback, invocation.arguments, invocation.count)
            .unwrap(),
        vec![LuaRequest::Notify("mapped=none".into())]
    );
}

#[test]
fn shipped_search_finder_and_tree_mappings_are_lua_owned() {
    let runtime = ConfigRuntime::build(&paths(Path::new("finder-maps")), None).unwrap();
    let mut resolver = KeyResolver::default();
    let plain = |character| KeyChord {
        code: KeyCode::Char(character),
        modifiers: KeyModifiers::NONE,
    };
    for (key, expected) in [
        ('/', "search.begin"),
        ('n', "search.next"),
        ('N', "search.previous"),
    ] {
        assert!(matches!(
            resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain(key)),
            ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == expected
        ));
    }
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "search.cancel"
    ));
    for key in [' ', 'f', 'f'] {
        let outcome = resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain(key));
        if key == 'f' && resolver.display().is_empty() {
            assert!(matches!(
                outcome,
                ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "finder.open"
            ));
        }
    }
    assert!(matches!(
        resolver.resolve(
            &runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::F(7),
                modifiers: KeyModifiers::ALT,
            },
        ),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "finder.open"
    ));
    resolver.reset();
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain(' ')),
        ResolveOutcome::Pending(_)
    ));
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain('t')),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "tree.toggle"
    ));
    assert!(matches!(
        resolver.resolve(&runtime.keymaps(), InputContext::Normal, plain('i')),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "edit.begin"
    ));
    for context in [
        "search",
        "finder_insert",
        "finder_searching",
        "finder_normal",
        "tree",
        "insert",
        "visual",
    ] {
        assert!(InputContext::parse(context).is_ok());
    }
}

#[test]
fn preview_options_theme_and_keymaps_are_typed_and_lua_owned() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        r#"
        wolf.option.set("preview.media_percent", 60)
        wolf.option.set("preview.documents.enabled", false)
        wolf.option.set("preview.documents.cache_pages", 5)
        wolf.option.set("preview.documents.max_render_pixels", 12000000)
        wolf.option.set("viewer.max_percent", 90)
        wolf.option.set("viewer.use_internal", false)
    "#,
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    assert_eq!(
        runtime.preview_options(),
        PreviewOptions {
            media_percent: 60,
            viewer_max_percent: 90,
            use_internal_image_viewer: false,
            documents: crate::preview::DocumentOptions {
                enabled: false,
                cache_pages: 5,
                max_render_pixels: 12_000_000,
            },
        }
    );
    let theme = runtime.theme().unwrap();
    assert_eq!(
        theme.preview_background,
        ratatui::style::Color::Rgb(0x1e, 0x1e, 0x2e)
    );
    assert_eq!(
        theme.preview_metadata_label,
        ratatui::style::Color::Rgb(0xf9, 0xe2, 0xaf)
    );

    let mut resolver = KeyResolver::default();
    for (context, chord, command) in [
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            },
            "preview.playback.toggle",
        ),
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::CONTROL,
            },
            "preview.audio.toggle_mute",
        ),
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            },
            "preview.metadata.page_down",
        ),
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            },
            "preview.metadata.page_up",
        ),
        (
            InputContext::Viewer,
            KeyChord {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
            },
            "viewer.close",
        ),
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char(']'),
                modifiers: KeyModifiers::NONE,
            },
            "preview.page.next",
        ),
        (
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char('['),
                modifiers: KeyModifiers::NONE,
            },
            "preview.page.previous",
        ),
        (
            InputContext::Viewer,
            KeyChord {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            },
            "viewer.close",
        ),
    ] {
        assert!(matches!(
            resolver.resolve(&runtime.keymaps(), context, chord),
            ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == command
        ));
    }

    let removed = TestDirectory::new();
    let removed_paths = paths(&removed.0);
    fs::create_dir_all(&removed_paths.config).unwrap();
    fs::write(
        removed_paths.config.join("init.lua"),
        "wolf.keymap.del('normal', ']')",
    )
    .unwrap();
    let removed_runtime = ConfigRuntime::load(removed_paths, false).unwrap();
    let mut removed_resolver = KeyResolver::default();
    assert!(matches!(
        removed_resolver.resolve(
            &removed_runtime.keymaps(),
            InputContext::Normal,
            KeyChord {
                code: KeyCode::Char(']'),
                modifiers: KeyModifiers::NONE,
            },
        ),
        ResolveOutcome::Unmapped { .. }
    ));

    let invalid = TestDirectory::new();
    let invalid_paths = paths(&invalid.0);
    fs::create_dir_all(&invalid_paths.config).unwrap();
    fs::write(
        invalid_paths.config.join("init.lua"),
        "wolf.option.set('preview.media_percent', 19)",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(invalid_paths, false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("preview.media_percent"))
    );
    assert_eq!(runtime.preview_options(), PreviewOptions::default());
}
