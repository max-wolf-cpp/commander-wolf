use crate::commands::CommandRegistry;

#[test]
fn completion_uses_sorted_live_public_aliases_and_target_descriptions() {
    let mut registry = CommandRegistry::with_builtins();
    assert!(registry.completions(":").is_empty());

    registry.alias("quit", "app.quit").unwrap();
    registry.alias("q", "app.quit").unwrap();
    registry.create_lua("custom.id", 7, "Custom description");
    registry.alias("custom", "custom.id").unwrap();

    let items = registry.completions(":");

    assert_eq!(
        items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["custom", "q", "quit"]
    );
    assert_eq!(items[0].description, "Custom description");
    assert_eq!(registry.completions(":qu")[0].name, "quit");
    assert_eq!(registry.completions(":q").len(), 2);
    assert!(registry.completions(":missing").is_empty());
    assert!(registry.completions(":quit ").is_empty());

    registry.alias("q", "custom.id").unwrap();
    assert_eq!(
        registry.completions(":q")[0].description,
        "Custom description"
    );

    registry.delete("custom.id").unwrap();
    assert_eq!(registry.completions(":").len(), 1);

    registry.delete_alias("quit");
    assert!(registry.completions(":").is_empty());
}

#[test]
fn fixed_argument_completion_uses_live_builtin_aliases_and_only_valid_positions() {
    let mut registry = CommandRegistry::with_builtins();
    for (alias, id) in [
        ("layout", "layout.set"),
        ("separator", "separator.set"),
        ("border", "border.set"),
        ("icons", "icons.set"),
        ("name-colors", "name_colors.set"),
        ("file-color-policy", "file_color_policy.set"),
    ] {
        registry.alias(alias, id).unwrap();
    }

    let names = |line: &str, registry: &CommandRegistry| {
        registry
            .completions(line)
            .into_iter()
            .map(|item| {
                assert!(item.description.is_empty());
                item.name
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(
        names(":separator ", &registry),
        [
            "dashed", "double", "off", "plain", "rounded", "simple", "thick"
        ]
    );
    assert_eq!(names(":border s", &registry), ["simple"]);
    assert_eq!(names(":icons n", &registry), ["nerd_font", "none"]);
    assert_eq!(names(":name-colors o", &registry), ["off", "on"]);
    assert_eq!(
        names(":file-color-policy ", &registry),
        ["nearest", "original"]
    );
    assert_eq!(
        names(":layout ", &registry),
        ["borderless", "compact", "framed"]
    );

    assert!(names(":layout compact ", &registry).is_empty());
    assert!(names(":icons nerd_font ", &registry).is_empty());

    registry.alias("i", "icons.set").unwrap();
    assert_eq!(names(":i u", &registry), ["unicode"]);

    registry.delete_alias("icons");
    assert!(names(":icons ", &registry).is_empty());

    registry.create_lua("custom", 3, "Custom");
    registry.alias("i", "custom").unwrap();
    assert!(names(":i ", &registry).is_empty());
}
