use super::*;

#[test]
fn invalid_user_syntax_theme_rolls_back_to_shipped_defaults() {
    let root = TestDirectory::new();
    let user_paths = paths(&root.0);
    fs::create_dir_all(&user_paths.config).unwrap();
    fs::write(
        user_paths.config.join("init.lua"),
        "wolf.syntax.use('missing')",
    )
    .unwrap();
    let runtime = ConfigRuntime::load(user_paths.clone(), false).unwrap();
    assert!(
        runtime
            .load_warning
            .as_ref()
            .is_some_and(|warning| warning.contains("unknown syntax preset"))
    );
    assert_eq!(
        runtime.syntax_theme().unwrap(),
        ConfigRuntime::load(user_paths, true)
            .unwrap()
            .syntax_theme()
            .unwrap()
    );
}

#[test]
fn global_and_required_api_are_identical() {
    let runtime = ConfigRuntime::build(&paths(Path::new("test")), None).unwrap();
    assert!(
        runtime
            .lua
            .load("return rawequal(wolf, require('commander_wolf'))")
            .eval::<bool>()
            .unwrap()
    );
}

#[test]
fn clean_keeps_shipped_defaults() {
    let runtime = ConfigRuntime::load(paths(Path::new("missing")), true).unwrap();
    assert_eq!(runtime.leader(), " ");
    assert!(runtime.draft.borrow().commands.resolve_alias("q").is_some());
    assert!(runtime.theme().is_ok());
}

#[test]
fn missing_user_configuration_is_normal() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    let runtime = ConfigRuntime::load(paths.clone(), false).unwrap();
    assert!(runtime.load_warning.is_none());
    assert!(!paths.config.exists());
}

#[test]
fn broken_user_config_rolls_back_without_copying_defaults() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    let init = paths.config.join("init.lua");
    fs::write(&init, "error('broken config')").unwrap();
    let runtime = ConfigRuntime::load(paths.clone(), false).unwrap();
    assert!(runtime.load_warning.as_ref().is_some_and(|warning| {
        warning.contains("broken config") && warning.contains("init.lua")
    }));
    assert!(runtime.draft.borrow().commands.resolve_alias("q").is_some());
    assert_eq!(fs::read_dir(&paths.config).unwrap().count(), 1);
}

#[test]
fn clean_does_not_execute_user_configuration() {
    let root = TestDirectory::new();
    let paths = paths(&root.0);
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(paths.config.join("init.lua"), "error('must not execute')").unwrap();
    let runtime = ConfigRuntime::load(paths, true).unwrap();
    assert!(runtime.load_warning.is_none());
}

#[test]
fn stdpath_uses_application_identifier() {
    let runtime = ConfigRuntime::build(&paths(Path::new("roots")), None).unwrap();
    let config = runtime
        .lua
        .load("return wolf.fn.stdpath('config')")
        .eval::<String>()
        .unwrap();
    assert!(std::path::Path::new(&config).ends_with("commander-wolf"));
}
