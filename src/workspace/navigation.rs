use super::ViewportState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionMotion {
    Next,
    Previous,
    First,
    Last,
    Absolute,
    ViewportTop,
    ViewportMiddle,
    ViewportBottom,
}

pub fn apply_selection_motion(
    selected: &mut usize,
    viewport: &mut ViewportState,
    row_count: usize,
    motion: SelectionMotion,
    count: Option<usize>,
    configured_scroll_offset: usize,
) {
    if row_count == 0 {
        *selected = 0;
        viewport.scroll = 0;
        return;
    }

    let last = row_count - 1;
    let repetitions = count.unwrap_or(1).max(1);

    *selected = match motion {
        SelectionMotion::Next => selected.saturating_add(repetitions).min(last),
        SelectionMotion::Previous => selected.saturating_sub(repetitions),
        SelectionMotion::First => count.map_or(0, |line| line.saturating_sub(1).min(last)),
        SelectionMotion::Last => count.map_or(last, |line| line.saturating_sub(1).min(last)),
        SelectionMotion::Absolute => count.unwrap_or(1).saturating_sub(1).min(last),
        SelectionMotion::ViewportTop => {
            viewport.top_index(last, configured_scroll_offset, repetitions)
        }
        SelectionMotion::ViewportMiddle => viewport.middle_index(last),
        SelectionMotion::ViewportBottom => {
            viewport.bottom_index(last, configured_scroll_offset, repetitions)
        }
    };

    viewport.ensure_visible(*selected, row_count, configured_scroll_offset);
}
