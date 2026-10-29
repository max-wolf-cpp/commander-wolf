use super::BuiltinCommand;
use crate::app::Action;
use crate::workspace::SelectionMotion;

// The exhaustive dispatcher routes only the matching selection family to each mapper.
pub(super) fn selection_action(command: BuiltinCommand, count: Option<usize>) -> Action {
    use BuiltinCommand::*;

    match command {
        SelectionNext => Action::MoveSelection {
            delta: 1,
            count: count.unwrap_or(1),
        },
        SelectionPrevious => Action::MoveSelection {
            delta: -1,
            count: count.unwrap_or(1),
        },
        SelectionFirst => Action::Select {
            motion: SelectionMotion::First,
            count,
        },
        SelectionLast => Action::Select {
            motion: SelectionMotion::Last,
            count,
        },
        SelectionAbsolute => Action::Select {
            motion: SelectionMotion::Absolute,
            count,
        },
        SelectionCenter => Action::CenterSelection,
        SelectionViewportTop => Action::Select {
            motion: SelectionMotion::ViewportTop,
            count,
        },
        SelectionViewportMiddle => Action::Select {
            motion: SelectionMotion::ViewportMiddle,
            count,
        },
        SelectionViewportBottom => Action::Select {
            motion: SelectionMotion::ViewportBottom,
            count,
        },
        _ => unreachable!("dispatcher routed an unsupported command to selection_action"),
    }
}

pub(super) fn finder_selection_action(command: BuiltinCommand, count: Option<usize>) -> Action {
    use BuiltinCommand::*;

    match command {
        FinderSelectionNext => Action::FinderSelect {
            motion: SelectionMotion::Next,
            count,
        },
        FinderSelectionPrevious => Action::FinderSelect {
            motion: SelectionMotion::Previous,
            count,
        },
        FinderSelectionFirst => Action::FinderSelect {
            motion: SelectionMotion::First,
            count,
        },
        FinderSelectionLast => Action::FinderSelect {
            motion: SelectionMotion::Last,
            count,
        },
        FinderSelectionCenter => Action::FinderCenterSelection,
        FinderSelectionViewportTop => Action::FinderSelect {
            motion: SelectionMotion::ViewportTop,
            count,
        },
        FinderSelectionViewportMiddle => Action::FinderSelect {
            motion: SelectionMotion::ViewportMiddle,
            count,
        },
        FinderSelectionViewportBottom => Action::FinderSelect {
            motion: SelectionMotion::ViewportBottom,
            count,
        },
        _ => unreachable!("dispatcher routed an unsupported command to finder_selection_action"),
    }
}

pub(super) fn tree_selection_action(command: BuiltinCommand, count: Option<usize>) -> Action {
    use BuiltinCommand::*;

    match command {
        TreeSelectionNext => Action::TreeSelect {
            motion: SelectionMotion::Next,
            count,
        },
        TreeSelectionPrevious => Action::TreeSelect {
            motion: SelectionMotion::Previous,
            count,
        },
        TreeSelectionFirst => Action::TreeSelect {
            motion: SelectionMotion::First,
            count,
        },
        TreeSelectionLast => Action::TreeSelect {
            motion: SelectionMotion::Last,
            count,
        },
        TreeSelectionCenter => Action::TreeCenterSelection,
        TreeSelectionViewportTop => Action::TreeSelect {
            motion: SelectionMotion::ViewportTop,
            count,
        },
        TreeSelectionViewportMiddle => Action::TreeSelect {
            motion: SelectionMotion::ViewportMiddle,
            count,
        },
        TreeSelectionViewportBottom => Action::TreeSelect {
            motion: SelectionMotion::ViewportBottom,
            count,
        },
        _ => unreachable!("dispatcher routed an unsupported command to tree_selection_action"),
    }
}
