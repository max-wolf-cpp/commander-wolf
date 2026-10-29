use crate::app::Action;
use crate::commands::{CommandDispatch, CommandId, CommandInvocation, CommandRegistry};

#[test]
fn aliases_dispatch_stable_builtin_ids() {
    let mut registry = CommandRegistry::with_builtins();
    registry.alias("q", "app.quit").unwrap();

    let id = registry.resolve_alias("q").unwrap();

    assert_eq!(
        registry
            .dispatch(CommandInvocation::new(id.clone(), Vec::new(), None))
            .unwrap(),
        CommandDispatch::Actions(vec![Action::Quit])
    );
}

#[test]
fn builtin_commands_preserve_argument_error_messages() {
    let registry = CommandRegistry::with_builtins();
    for (id, usage) in [
        ("panel.refresh", ":refresh"),
        ("file.copy_to_other", ":copy"),
        ("file.move_to_other", ":move"),
        ("file.delete", ":delete"),
        ("file.reveal_selected", "file.reveal_selected"),
        ("file.rename_prompt", "file.rename_prompt"),
        ("confirmation.accept", "confirmation.accept"),
        ("selection.next", "selection.next"),
    ] {
        let error = registry
            .dispatch(CommandInvocation::new(
                CommandId::from(id),
                vec!["extra".into()],
                None,
            ))
            .unwrap_err();

        assert_eq!(
            error,
            format!("{id} usage: {usage}; this command does not accept arguments")
        );
    }
}

#[test]
fn lua_dispatch_preserves_the_callback_and_invocation() {
    let mut registry = CommandRegistry::with_builtins();
    registry.create_lua("custom", 7, "Custom command");
    registry.alias("c", "custom").unwrap();

    let invocation = CommandInvocation::new(
        registry.resolve_alias("c").unwrap().clone(),
        vec!["目录 with spaces".into()],
        Some(3),
    );

    assert_eq!(
        registry.dispatch(invocation.clone()).unwrap(),
        CommandDispatch::Lua {
            callback: 7,
            invocation,
        }
    );
}
