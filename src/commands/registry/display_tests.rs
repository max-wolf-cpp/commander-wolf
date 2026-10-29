use crate::app::Action;
use crate::commands::{CommandDispatch, CommandId, CommandInvocation, CommandRegistry};
use crate::icons::IconMode;
use crate::status_line::UiPreset;
use crate::theme::{BorderStyle, SeparatorStyle};

#[test]
fn session_display_commands_validate_fixed_arguments() {
    let registry = CommandRegistry::with_builtins();
    for (id, value, action) in [
        (
            "separator.set",
            "double",
            Action::SetSeparatorStyle(SeparatorStyle::Double),
        ),
        (
            "border.set",
            "simple",
            Action::SetBorderStyle(BorderStyle::Simple),
        ),
        (
            "icons.set",
            "unicode",
            Action::SetIconMode(IconMode::Unicode),
        ),
        ("name_colors.set", "on", Action::SetNameColors(true)),
        ("name_colors.set", "off", Action::SetNameColors(false)),
    ] {
        let dispatch = |arguments: Vec<String>| {
            registry.dispatch(CommandInvocation::new(
                CommandId(id.into()),
                arguments,
                None,
            ))
        };

        assert_eq!(
            dispatch(vec![value.into()]).unwrap(),
            CommandDispatch::Actions(vec![action])
        );
        assert!(dispatch(vec![]).is_err());
        assert!(dispatch(vec![value.into(), "extra".into()]).is_err());
    }

    for (id, value) in [
        ("separator.set", "heavy_double_dashed"),
        ("border.set", "mystery"),
        ("icons.set", "emoji"),
        ("name_colors.set", "toggle"),
    ] {
        assert!(
            registry
                .dispatch(CommandInvocation::new(
                    CommandId(id.into()),
                    vec![value.into()],
                    None,
                ))
                .is_err()
        );
    }
}

#[test]
fn layout_commands_validate_presets_and_arity() {
    assert_eq!(
        dispatch("layout.set", vec!["framed"]).unwrap(),
        CommandDispatch::Actions(vec![Action::SetStylePreset(UiPreset::Framed)])
    );
    assert_eq!(
        dispatch("layout.set", vec!["borderless"]).unwrap(),
        CommandDispatch::Actions(vec![Action::SetStylePreset(UiPreset::Borderless)])
    );
    assert_eq!(
        dispatch("layout.set", vec!["compact"]).unwrap(),
        CommandDispatch::Actions(vec![Action::SetStylePreset(UiPreset::Compact)])
    );

    assert!(
        dispatch("layout.set", vec!["cornered"])
            .unwrap_err()
            .contains("style preset must be")
    );
    assert!(
        dispatch("layout.set", vec!["top", "framed"])
            .unwrap_err()
            .contains(":layout")
    );
    assert!(
        dispatch("layout.set", vec![])
            .unwrap_err()
            .contains(":layout")
    );
}

#[test]
fn named_display_commands_require_one_name() {
    assert_eq!(
        dispatch("display_style.use", vec!["far"]).unwrap(),
        CommandDispatch::Actions(vec![Action::UseDisplayStyle("far".into())])
    );
    assert_eq!(
        dispatch("theme.use", vec!["default"]).unwrap(),
        CommandDispatch::Actions(vec![Action::UseTheme("default".into())])
    );

    assert!(dispatch("theme.use", vec![]).is_err());
    assert!(dispatch("theme.use", vec!["default", "extra"]).is_err());
}

fn dispatch(id: &str, arguments: Vec<&str>) -> Result<CommandDispatch, String> {
    CommandRegistry::with_builtins().dispatch(CommandInvocation::new(
        CommandId::from(id),
        arguments.into_iter().map(str::to_owned).collect(),
        None,
    ))
}
