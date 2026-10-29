use super::*;

#[test]
fn command_completion_dismisses_edits_and_submits_without_recording_tab_navigation() {
    use crate::commands::completion::CompletionItem;

    let mut state = state();
    let candidates = ["copy", "count"]
        .map(|name| CompletionItem {
            name: name.into(),
            description: String::new(),
        })
        .to_vec();

    assert!(update(&mut state, Action::CompleteCommand { backwards: false }).is_empty());
    update(&mut state, Action::EnterCommandMode);
    state.command_line = ":co".into();
    assert_eq!(
        update(&mut state, Action::CompleteCommand { backwards: false }),
        vec![Effect::CompleteCommand {
            line: ":co".into(),
            backwards: false
        }]
    );

    let open = Action::CommandCompletionsLoaded {
        line: ":co".into(),
        items: candidates,
        backwards: false,
    };
    update(&mut state, open.clone());
    assert_eq!(state.command_line, ":copy");
    update(&mut state, Action::CompleteCommand { backwards: true });
    assert_eq!(state.command_line, ":count");

    update(&mut state, Action::CancelCommandMode);
    assert_eq!(state.mode, AppMode::Command);
    assert_eq!(state.command_line, ":co");
    assert!(state.command_completion.is_none());
    assert!(state.command_history.entries().is_empty());

    for action in [
        Action::PushCommandCharacter(' '),
        Action::DeleteCommandCharacter,
        Action::PreviousCommandHistory,
        Action::NextCommandHistory,
    ] {
        state.command_line = ":co".into();
        update(&mut state, open.clone());
        update(&mut state, action);
        assert!(state.command_completion.is_none());
    }

    state.command_line = ":changed".into();
    update(&mut state, open.clone());
    assert!(state.command_completion.is_none());

    state.command_line = ":co".into();
    update(&mut state, open);
    let effects = update(&mut state, Action::SubmitCommand);
    assert!(effects.contains(&Effect::ExecuteCommandLine(":copy".into())));
    assert!(state.command_completion.is_none());
    assert_eq!(state.command_history.entries(), &[":copy"]);
    assert_eq!(state.mode, AppMode::Normal);
}

#[test]
fn completion_temporarily_clears_and_restores_native_preview_images() {
    use crate::commands::completion::CompletionItem;

    let mut state = state();
    state.workspace.preview_visible = true;
    state.preview.path = Some("a.png".into());
    state.preview.media = LoadState::Loading;

    update(&mut state, Action::EnterCommandMode);
    let effects = update(
        &mut state,
        Action::CommandCompletionsLoaded {
            line: ":".into(),
            backwards: false,
            items: vec![CompletionItem {
                name: "copy".into(),
                description: String::new(),
            }],
        },
    );
    assert_eq!(effects, vec![Effect::ClearMediaSurface]);

    let effects = update(&mut state, Action::CancelCommandMode);
    assert!(
        matches!(effects.as_slice(), [Effect::PreparePreviewMedia { path, .. }] if path == std::path::Path::new("a.png"))
    );
}

#[test]
fn command_history_records_submissions_but_not_cancelled_drafts() {
    let mut state = state();
    update(&mut state, Action::EnterCommandMode);
    state.command_line = ":unknown 狼".into();
    assert_eq!(
        update(&mut state, Action::SubmitCommand),
        vec![Effect::ExecuteCommandLine(":unknown 狼".into())]
    );
    assert_eq!(state.command_history.entries(), &[":unknown 狼"]);

    update(&mut state, Action::EnterCommandMode);
    state.command_line = ":cd draft".into();
    assert!(update(&mut state, Action::PreviousCommandHistory).is_empty());
    assert_eq!(state.command_line, ":unknown 狼");
    update(&mut state, Action::PushCommandCharacter('x'));
    assert_eq!(state.command_history.entries(), &[":unknown 狼"]);
    update(&mut state, Action::NextCommandHistory);
    assert_eq!(state.command_line, ":cd draft");

    update(&mut state, Action::CancelCommandMode);
    update(&mut state, Action::EnterCommandMode);
    update(&mut state, Action::NextCommandHistory);
    assert_eq!(state.command_line, ":");
    update(&mut state, Action::PreviousCommandHistory);
    assert_eq!(state.command_line, ":unknown 狼");

    update(&mut state, Action::SubmitCommand);
    assert_eq!(state.command_history.entries().len(), 1);
    update(&mut state, Action::EnterCommandMode);
    update(&mut state, Action::SubmitCommand);
    assert_eq!(state.command_history.entries().len(), 1);

    state.command_line = ":untouched".into();
    update(&mut state, Action::PreviousCommandHistory);
    assert_eq!(state.command_line, ":untouched");
}
