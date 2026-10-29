use crate::app::Action;
use crate::commands::{CommandDispatch, CommandId, CommandInvocation, CommandRegistry};
use crate::search::SearchDirection;
use crate::workspace::SelectionMotion;

#[test]
fn search_and_finder_ids_are_stable_and_navigation_owns_count_policy() {
    let registry = CommandRegistry::with_builtins();
    for id in [
        "search.begin",
        "search.submit",
        "search.cancel",
        "search.backspace",
        "search.next",
        "search.previous",
        "finder.open",
        "finder.cancel",
        "finder.submit",
        "finder.backspace",
        "finder.enter_insert",
        "finder.open_selected",
        "finder.selection.next",
        "finder.selection.previous",
        "finder.selection.first",
        "finder.selection.last",
        "finder.selection.center",
        "finder.selection.viewport_top",
        "finder.selection.viewport_middle",
        "finder.selection.viewport_bottom",
        "preview.metadata.page_down",
        "preview.metadata.page_up",
        "preview.page.next",
        "preview.page.previous",
        "file.open_selected",
        "file.reveal_selected",
        "editor.open_cwd",
        "viewer.open_selected",
        "viewer.close",
    ] {
        assert!(registry.contains(id));
    }

    assert_eq!(
        registry
            .dispatch(CommandInvocation::new(
                CommandId("search.next".into()),
                Vec::new(),
                Some(5),
            ))
            .unwrap(),
        CommandDispatch::Actions(vec![Action::SearchRepeat {
            direction: SearchDirection::Forward,
            count: 5,
        }])
    );

    assert_eq!(
        registry
            .dispatch(CommandInvocation::new(
                CommandId("preview.page.next".into()),
                Vec::new(),
                Some(3),
            ))
            .unwrap(),
        CommandDispatch::Actions(vec![Action::PreviewPageNext { count: 3 }])
    );

    assert_eq!(
        registry
            .dispatch(CommandInvocation::new(
                CommandId("finder.selection.last".into()),
                Vec::new(),
                Some(3),
            ))
            .unwrap(),
        CommandDispatch::Actions(vec![Action::FinderSelect {
            motion: SelectionMotion::Last,
            count: Some(3),
        }])
    );
}
