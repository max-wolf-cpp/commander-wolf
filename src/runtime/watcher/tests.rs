use std::fs;
use std::path::Path;
use std::thread;

use super::events::event_affects;
use super::refresh::{load_watched_snapshot, refresh_directories};
use super::*;
use crate::terminal::TerminalSize;
use crate::test_support::TempDirectory;
use crate::workspace::WorkspaceOptions;

#[test]
fn watches_external_changes_without_navigation() {
    let root = TempDirectory::new();
    let mut state = AppState::new(
        root.0.clone(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );

    let snapshot = crate::filesystem::load_snapshot(&root.0, None, 0).unwrap();
    let scroll_offset = state.workspace.scroll_offset;

    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).replace_directory(
            snapshot.directory.clone(),
            snapshot.entries.clone(),
            snapshot.selected,
            scroll_offset,
        );
    }

    let mut watcher = DirectoryWatcher::new().unwrap();
    watcher.synchronize(&state);
    let initial_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        watcher.poll(&state);
        if watcher.due_at.is_none() && watcher.worker.is_none() && watcher.completed.is_none() {
            break;
        }

        assert!(
            Instant::now() < initial_deadline,
            "initial scan did not finish"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let created = root.join("created.txt");
    fs::write(&created, "external change").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        watcher.synchronize(&state);
        if let Some(Action::WatchedPanelsRefreshed { results, .. }) = watcher.poll(&state) {
            assert!(results.iter().all(|result| {
                result.result.as_ref().is_ok_and(|snapshot| {
                    snapshot.entries.iter().any(|entry| entry.path == created)
                })
            }));
            break;
        }

        assert!(
            Instant::now() < deadline,
            "external create was not observed"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn deleted_panel_directory_falls_back_to_existing_parent() {
    let root = TempDirectory::new();
    let child = root.join("child");
    fs::create_dir(&child).unwrap();
    fs::remove_dir(&child).unwrap();

    let snapshot = load_watched_snapshot(&child).unwrap();
    assert_eq!(snapshot.directory, root.0);
}

#[test]
fn unrelated_sibling_events_do_not_refresh_panel() {
    let directories = [PathBuf::from("/root/left"), PathBuf::from("/root/right")];
    assert!(event_affects(
        Path::new("/root/left/new.txt"),
        &directories,
        None
    ));
    assert!(event_affects(Path::new("/root/right"), &directories, None));
    assert!(!event_affects(
        Path::new("/root/sibling/new.txt"),
        &directories,
        None
    ));
}

#[test]
fn stale_worker_result_is_discarded_after_a_panel_refresh() {
    let mut state = AppState::new(
        PathBuf::from("."),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );

    for panel in [PanelId::Left, PanelId::Right] {
        state.workspace.panel_mut(panel).replace_directory(
            PathBuf::from("current"),
            Vec::new(),
            0,
            0,
        );
    }

    let mut watcher = DirectoryWatcher::new().unwrap();
    watcher.completed = Some(refresh_directories(
        [PathBuf::from("old"), PathBuf::from("old")],
        vec![PathBuf::from("old/file")],
        [0, 0],
    ));

    assert!(watcher.poll(&state).is_none());
    assert!(watcher.completed.is_none());
    assert!(watcher.pending_paths.contains(Path::new("old/file")));
    assert!(watcher.due_at.is_some());
}

#[test]
fn event_bursts_keep_only_bounded_path_state() {
    let mut watcher = DirectoryWatcher::new().unwrap();
    let now = Instant::now();
    for index in 0..1_000 {
        watcher.record_path(PathBuf::from(format!("entry-{index}")), now);
    }

    assert!(watcher.pending_all);
    assert!(watcher.pending_paths.is_empty());
    assert!(watcher.due_at.is_some());
}
