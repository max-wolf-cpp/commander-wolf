use super::*;
use crate::commands::{CommandId, CommandInvocation};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn chord(character: char) -> KeyChord {
    KeyChord {
        code: KeyCode::Char(character),
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn resolves_counts_and_multi_key_bindings() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "gg", "selection.first", " ")
        .unwrap();
    let mut resolver = KeyResolver::default();
    assert_eq!(
        resolver.resolve(&maps, InputContext::Normal, chord('1')),
        ResolveOutcome::Pending("1".into())
    );
    assert_eq!(
        resolver.resolve(&maps, InputContext::Normal, chord('2')),
        ResolveOutcome::Pending("12".into())
    );
    assert_eq!(
        resolver.resolve(&maps, InputContext::Normal, chord('g')),
        ResolveOutcome::Pending("12g".into())
    );
    assert_eq!(
        resolver.resolve(&maps, InputContext::Normal, chord('g')),
        ResolveOutcome::Command(CommandInvocation::new(
            CommandId("selection.first".into()),
            Vec::new(),
            Some(12)
        ))
    );
}

#[test]
fn visual_navigation_accepts_vim_counts() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Visual, "j", "selection.next", " ")
        .unwrap();
    let mut resolver = KeyResolver::default();
    assert_eq!(
        resolver.resolve(&maps, InputContext::Visual, chord('5')),
        ResolveOutcome::Pending("5".into())
    );
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Visual, chord('j')),
        ResolveOutcome::Command(CommandInvocation { count: Some(5), .. })
    ));
}

#[test]
fn later_mapping_overrides_the_same_sequence() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "j", "old", " ").unwrap();
    maps.set(InputContext::Normal, "j", "new", " ").unwrap();
    let mut resolver = KeyResolver::default();
    assert!(
        matches!(resolver.resolve(&maps, InputContext::Normal, chord('j')), ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "new")
    );
}

#[test]
fn descriptions_follow_binding_replacement_and_removal() {
    let mut maps = Keymaps::default();
    maps.set_with_description(
        InputContext::Normal,
        "j",
        "old",
        Some("Old action".into()),
        " ",
    )
    .unwrap();
    assert_eq!(
        maps.bindings(InputContext::Normal)[0]
            .description
            .as_deref(),
        Some("Old action")
    );

    maps.set_with_description(
        InputContext::Normal,
        "j",
        "new",
        Some("New action".into()),
        " ",
    )
    .unwrap();
    let bindings = maps.bindings(InputContext::Normal);
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].command, CommandId("new".into()));
    assert_eq!(bindings[0].description.as_deref(), Some("New action"));

    maps.set(InputContext::Normal, "j", "plain", " ").unwrap();
    assert_eq!(maps.bindings(InputContext::Normal)[0].description, None);
    maps.delete(InputContext::Normal, "j", " ").unwrap();
    assert!(maps.bindings(InputContext::Normal).is_empty());

    maps.set_with_description(InputContext::Normal, "k", "up", Some("Move up".into()), " ")
        .unwrap();
    maps.clear(InputContext::Normal);
    assert!(maps.bindings(InputContext::Normal).is_empty());
}

#[test]
fn binding_key_labels_include_leader_and_special_keys() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "<Leader>ff", "finder.open", " ")
        .unwrap();
    maps.set(InputContext::Insert, "<Left>", "edit.cursor.left", " ")
        .unwrap();
    maps.set(InputContext::Insert, "<Home>", "edit.cursor.home", " ")
        .unwrap();
    assert_eq!(
        maps.bindings(InputContext::Normal)[0].display_keys(),
        "<Space>ff"
    );
    assert_eq!(
        maps.bindings(InputContext::Insert)[0].display_keys(),
        "<Left>"
    );
    assert_eq!(
        maps.bindings(InputContext::Insert)[1].display_keys(),
        "<Home>"
    );
}

#[test]
fn invalid_continuation_resets_pending_state() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "gg", "top", " ").unwrap();
    let mut resolver = KeyResolver::default();
    resolver.resolve(&maps, InputContext::Normal, chord('g'));
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord(':')),
        ResolveOutcome::Unmapped {
            consumed_prefix: true,
            ..
        }
    ));
    assert!(resolver.display().is_empty());
}

#[test]
fn count_is_preserved_for_commands_and_invalid_prefix_is_consumed() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "gg", "top", " ").unwrap();
    maps.set(InputContext::Normal, ":", "command.enter", " ")
        .unwrap();
    let mut resolver = KeyResolver::default();
    resolver.resolve(&maps, InputContext::Normal, chord('5'));
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord(':')),
        ResolveOutcome::Command(CommandInvocation { count: Some(5), .. })
    ));
    assert!(resolver.display().is_empty());

    resolver.resolve(&maps, InputContext::Normal, chord('g'));
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord('h')),
        ResolveOutcome::Unmapped {
            consumed_prefix: true,
            ..
        }
    ));
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord(':')),
        ResolveOutcome::Command(_)
    ));
}

#[test]
fn delete_and_clear_remove_bindings() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "j", "next", " ").unwrap();
    maps.delete(InputContext::Normal, "j", " ").unwrap();
    let mut resolver = KeyResolver::default();
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord('j')),
        ResolveOutcome::Unmapped { .. }
    ));
    maps.set(InputContext::Normal, "j", "next", " ").unwrap();
    maps.clear(InputContext::Normal);
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord('j')),
        ResolveOutcome::Unmapped { .. }
    ));
}

#[test]
fn leader_uses_the_configured_sequence() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "<Leader>q", "quit", ",")
        .unwrap();
    let mut resolver = KeyResolver::default();
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, chord(',')),
        ResolveOutcome::Pending(_)
    ));
    assert!(
        matches!(resolver.resolve(&maps, InputContext::Normal, chord('q')), ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "quit")
    );
}

#[test]
fn shifted_printable_character_is_normalized_by_its_text() {
    let event = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::SHIFT);
    assert_eq!(
        KeyChord::from(event),
        KeyChord {
            code: KeyCode::Char(':'),
            modifiers: KeyModifiers::NONE
        }
    );
}

#[test]
fn parses_shifted_function_keys_and_confirm_context_generically() {
    let mut maps = Keymaps::default();
    maps.set(InputContext::Normal, "<S-F8>", "file.delete", " ")
        .unwrap();
    maps.set(InputContext::Confirm, "y", "confirmation.accept", " ")
        .unwrap();
    let mut resolver = KeyResolver::default();
    let delete = KeyChord {
        code: KeyCode::F(8),
        modifiers: KeyModifiers::SHIFT,
    };
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Normal, delete),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "file.delete"
    ));
    assert!(matches!(
        resolver.resolve(&maps, InputContext::Confirm, chord('y')),
        ResolveOutcome::Command(CommandInvocation { id: CommandId(id), .. }) if id == "confirmation.accept"
    ));
}
