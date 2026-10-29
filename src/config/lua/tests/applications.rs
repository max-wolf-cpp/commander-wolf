use super::*;

#[test]
fn shell_commands_preserve_source_and_accept_an_explicit_shell() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(
        paths.config.join("init.lua"),
        "wolf.option.set('shell', { program = 'custom-shell', args = { '-c' } })",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(paths.clone(), false).unwrap();
    assert_eq!(
        runtime.shell(),
        Some(Application {
            program: "custom-shell".into(),
            arguments: vec!["-c".into()]
        })
    );
    let source = "echo 'a b' | command > output.txt";
    assert_eq!(
        runtime.command_line(&format!(":!{source}")).unwrap(),
        CommandDispatch::Actions(vec![crate::app::Action::RunShellCommand(source.into())])
    );
    assert!(
        runtime
            .command_completions(":")
            .iter()
            .any(|item| item.name == "!")
    );
    assert!(
        ConfigRuntime::load(paths.clone(), true)
            .unwrap()
            .shell()
            .is_none()
    );
    fs::write(
        paths.config.join("init.lua"),
        "wolf.option.set('shell', 123)",
    )
    .unwrap();
    assert!(
        ConfigRuntime::load(paths, false)
            .unwrap()
            .load_warning
            .is_some()
    );
}

#[test]
fn file_associations_accept_one_extension_or_a_list_with_typed_arguments() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        r#"
            wolf.file_association.set("md", "markdown-viewer")
            wolf.file_association.set({ "jpg", ".JPEG" }, {
                program = "image editor",
                args = { "--reuse-window" },
            })
            wolf.file_association.set_text_editor({
                program = "nvim",
                args = { "--" },
            })
            wolf.file_association.del("jpeg")
        "#,
    )
    .unwrap();

    let runtime = ConfigRuntime::load(user_paths, false).unwrap();
    let associations = runtime.file_associations();
    assert_eq!(
        associations
            .application_for(Path::new("README.MD"))
            .unwrap()
            .program,
        "markdown-viewer"
    );
    let image = associations.application_for(Path::new("wolf.jpg")).unwrap();
    assert_eq!(image.program, "image editor");
    assert_eq!(image.arguments, ["--reuse-window"]);
    assert!(
        associations
            .application_for(Path::new("wolf.jpeg"))
            .is_none()
    );
    let editor = associations.text_editor().unwrap();
    assert_eq!(editor.program, "nvim");
    assert_eq!(editor.arguments, ["--"]);
}
