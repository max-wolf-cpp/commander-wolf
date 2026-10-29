use super::playback::stop_video;
use super::preview::{clear_preview, prepare_finder_selected, prepare_selected};
use crate::app::{Action, AppMode, AppState, Effect};
use crate::finder::{FinderMode, FinderSearchRequest, GlobMatcher};
use crate::workspace::PanelId;

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::FinderOpen => open_finder(state),
        Action::FinderCancel => cancel_finder(state),
        Action::FinderPushCharacter(character) => {
            if state.finder.mode == FinderMode::Insert {
                state.finder.pattern.push(character);
                state.finder.error = None;
            }
            Vec::new()
        }
        Action::FinderBackspace => {
            if state.finder.mode == FinderMode::Insert {
                state.finder.pattern.pop();
                state.finder.error = None;
            }
            Vec::new()
        }
        Action::FinderSubmit => submit_finder(state),
        Action::FinderEnterInsert => {
            if state.finder.mode == FinderMode::Normal {
                state.finder.mode = FinderMode::Insert;
                state.finder.error = None;
            }
            Vec::new()
        }
        Action::FinderSelect { motion, count } => {
            if state.finder.mode == FinderMode::Normal {
                let previous = state
                    .finder
                    .selected_result()
                    .map(|result| result.absolute_path.clone());

                state
                    .finder
                    .apply_motion(motion, count, state.finder_options.scroll_offset);

                if state
                    .finder
                    .selected_result()
                    .is_some_and(|result| Some(&result.absolute_path) != previous.as_ref())
                {
                    return prepare_finder_selected(state);
                }
            }
            Vec::new()
        }
        Action::FinderCenterSelection => {
            if state.finder.mode == FinderMode::Normal {
                state.finder.center_selection();
            }
            Vec::new()
        }
        Action::FinderOpenSelected => reveal_finder_selection(state),
        Action::FinderResultsAdded {
            generation,
            results,
        } => {
            if state.finder.mode == FinderMode::Searching && generation == state.finder.generation {
                let was_empty = state.finder.results.is_empty();
                state.finder.results.extend(results);
                if was_empty && !state.finder.results.is_empty() {
                    return prepare_finder_selected(state);
                }
            }
            Vec::new()
        }
        Action::FinderCompleted {
            generation,
            skipped,
            truncated,
            cancelled: _,
        } => {
            if state.finder.mode == FinderMode::Searching && generation == state.finder.generation {
                state.finder.skipped = skipped;
                state.finder.truncated = truncated;
                state.finder.finalize();
            }
            Vec::new()
        }
        Action::FinderFailed {
            generation,
            message,
        } => {
            if state.finder.mode == FinderMode::Searching && generation == state.finder.generation {
                state.finder.mode = FinderMode::Normal;
                state.finder.error = Some(message);
                state.finder.finalize();
            }
            Vec::new()
        }
        Action::FinderRevealLoaded {
            generation,
            panel,
            directory,
            entries,
            selected,
        } => reveal_finder_loaded(state, generation, panel, directory, entries, selected),
        Action::FinderRevealFailed {
            generation,
            message,
        } => {
            if state.finder.is_open() && generation == state.finder.generation {
                state.finder.error = Some(message);
            }
            Vec::new()
        }
        _ => unreachable!("dispatcher routed an unsupported action to finder"),
    }
}

fn open_finder(state: &mut AppState) -> Vec<Effect> {
    if state.mode != AppMode::Normal || state.finder.is_open() || state.directory_tree.open {
        return Vec::new();
    }

    let panel = state.workspace.active_panel;
    let root = state.workspace.active().directory.clone();

    let mut effects = stop_video(state);
    effects.extend(clear_preview(state));

    state.preview.clear();
    state.finder.open(panel, root);
    state.finder.set_visible_rows(
        crate::ui::finder_visible_rows(state.terminal_size.area()),
        state.finder_options.scroll_offset,
    );
    state.pending_keys.clear();

    effects
}

fn cancel_finder(state: &mut AppState) -> Vec<Effect> {
    match state.finder.mode {
        FinderMode::Closed => Vec::new(),
        FinderMode::Insert if !state.finder.results.is_empty() => {
            state.finder.mode = FinderMode::Normal;
            Vec::new()
        }
        FinderMode::Searching => {
            state.finder.finalize();
            vec![Effect::CancelFinderSearch]
        }
        FinderMode::Insert | FinderMode::Normal => close_finder(state),
    }
}

fn close_finder(state: &mut AppState) -> Vec<Effect> {
    state.finder.close();
    state.preview.clear();
    state.pending_keys.clear();

    let mut effects = vec![
        Effect::CancelFinderSearch,
        Effect::ClearMediaSurface,
        Effect::CancelDocumentPreview,
    ];
    effects.extend(prepare_selected(state));
    effects
}

fn submit_finder(state: &mut AppState) -> Vec<Effect> {
    if state.finder.mode != FinderMode::Insert {
        return Vec::new();
    }

    let matcher = match GlobMatcher::parse(&state.finder.pattern) {
        Ok(matcher) => matcher,
        Err(error) => {
            state.finder.error = Some(error);
            return Vec::new();
        }
    };

    state.finder.generation = state.finder.generation.wrapping_add(1).max(1);
    state.finder.results.clear();
    state.finder.selected = 0;
    state.finder.viewport.scroll = 0;
    state.finder.skipped = 0;
    state.finder.truncated = false;
    state.finder.error = None;
    state.finder.mode = FinderMode::Searching;
    state.preview.clear();

    vec![
        Effect::ClearMediaSurface,
        Effect::CancelDocumentPreview,
        Effect::StartFinderSearch(FinderSearchRequest {
            generation: state.finder.generation,
            root: state.finder.root.clone(),
            matcher,
            options: state.finder_options,
        }),
    ]
}

fn reveal_finder_selection(state: &mut AppState) -> Vec<Effect> {
    if state.finder.mode != FinderMode::Normal {
        return Vec::new();
    }

    let Some(path) = state
        .finder
        .selected_result()
        .map(|result| result.absolute_path.clone())
    else {
        return Vec::new();
    };

    state.finder.error = None;

    vec![Effect::RevealFinderResult {
        generation: state.finder.generation,
        panel: state.finder.panel,
        path,
    }]
}

fn reveal_finder_loaded(
    state: &mut AppState,
    generation: u64,
    panel: PanelId,
    directory: std::path::PathBuf,
    entries: Vec<crate::workspace::DirectoryEntry>,
    selected: usize,
) -> Vec<Effect> {
    if !state.finder.is_open()
        || generation != state.finder.generation
        || panel != state.finder.panel
    {
        return Vec::new();
    }

    let scroll_offset = state.workspace.scroll_offset;
    state
        .workspace
        .panel_mut(panel)
        .replace_directory(directory, entries, selected, scroll_offset);
    state.search.clear_visuals_for(panel);

    close_finder(state)
}
