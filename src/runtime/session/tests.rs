use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use super::format::{HEADER, MAX_BYTES, decode, decode_path, encode_path};
use super::*;
use crate::workspace::WorkspaceOptions;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "wolf-session-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn workspace(&self) -> WorkspaceState {
        let mut workspace = WorkspaceState::new(WorkspaceOptions::default());
        for (panel, name) in workspace.panels.iter_mut().zip(["Левая 狼", "Right"]) {
            panel.directory = self.0.join(name);
            fs::create_dir_all(&panel.directory).unwrap();
        }
        workspace
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn saves_restores_and_replaces_both_panel_paths() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.join("State"));
    let mut workspace = root.workspace();
    store.save(true, &workspace).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap(),
        [
            workspace.panels[0].directory.clone(),
            workspace.panels[1].directory.clone()
        ]
    );

    let actions = store.startup_actions(&root.0, true);
    assert_eq!(actions.len(), 2);
    for (action, panel) in actions.iter().zip([PanelId::Left, PanelId::Right]) {
        assert!(
            matches!(action, Action::DirectoryLoaded { panel: actual, directory, .. }
            if *actual == panel && *directory == workspace.panel(panel).directory)
        );
    }

    workspace.panels.swap(0, 1);
    store.save(true, &workspace).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap()[0],
        workspace.panels[0].directory
    );
    assert_eq!(
        fs::read_dir(store.path.parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn disabled_session_never_creates_reads_or_modifies_state() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.join("State"));
    let workspace = root.workspace();
    store.save(false, &workspace).unwrap();
    assert!(!store.path.parent().unwrap().exists());

    fs::create_dir(store.path.parent().unwrap()).unwrap();
    fs::write(&store.path, b"corrupt existing session").unwrap();
    assert!(matches!(
        store.startup_actions(&root.0, false).as_slice(),
        [Action::Initialize]
    ));
    store.save(false, &workspace).unwrap();
    assert_eq!(fs::read(&store.path).unwrap(), b"corrupt existing session");
}

#[test]
fn missing_and_corrupt_sessions_preserve_normal_startup() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.clone());
    assert!(matches!(
        store.startup_actions(&root.0, true).as_slice(),
        [Action::Initialize]
    ));

    fs::write(&store.path, b"invalid").unwrap();
    assert!(matches!(
        store.startup_actions(&root.0, true).as_slice(),
        [Action::Initialize, Action::Notify(_)]
    ));
}

#[test]
fn unavailable_directory_falls_back_only_for_its_panel() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.clone());
    let workspace = root.workspace();
    store.save(true, &workspace).unwrap();

    fs::remove_dir(&workspace.panels[0].directory).unwrap();
    // A former directory becoming a file must not be restored as its parent.
    fs::write(&workspace.panels[0].directory, b"now a file").unwrap();

    let actions = store.startup_actions(&root.0, true);
    assert!(
        matches!(&actions[0], Action::DirectoryLoaded { panel: PanelId::Left, directory, .. } if *directory == root.0)
    );
    assert!(matches!(&actions[1], Action::Notify(_)));
    assert!(
        matches!(&actions[2], Action::DirectoryLoaded { panel: PanelId::Right, directory, .. } if *directory == workspace.panels[1].directory)
    );

    fs::remove_file(&workspace.panels[0].directory).unwrap();
    assert!(
        matches!(&store.startup_actions(&root.0, true)[0], Action::DirectoryLoaded { directory, .. } if *directory == root.0)
    );
}

#[test]
fn invalid_records_are_rejected_without_panics() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.clone());
    store.save(true, &root.workspace()).unwrap();
    let bytes = fs::read(&store.path).unwrap();
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }

    let mut trailing = bytes.clone();
    trailing.push(1);
    assert!(decode(&trailing).is_err());

    let mut oversized_length = bytes;
    oversized_length[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode(&oversized_length).is_err());
    assert!(decode(&vec![0; MAX_BYTES + 1]).is_err());

    let relative = encode_path(Path::new("relative"));
    let mut record = HEADER.to_vec();
    record.extend_from_slice(&(relative.len() as u32).to_le_bytes());
    record.extend_from_slice(&relative);
    assert!(decode(&record).is_err());
}

#[test]
fn failed_save_reports_error_and_preserves_existing_state() {
    let root = TestDirectory::new();
    let store = SessionStore::new(root.0.clone());
    fs::create_dir(&store.path).unwrap();
    fs::write(store.path.join("keep"), b"keep").unwrap();
    assert!(
        store
            .save(true, &root.workspace())
            .unwrap_err()
            .contains("Cannot save session")
    );
    assert_eq!(fs::read(store.path.join("keep")).unwrap(), b"keep");
    assert!(!fs::read_dir(&root.0).unwrap().any(|entry| {
        entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "tmp")
    }));
}

#[test]
fn native_non_unicode_path_encoding_round_trips() {
    #[cfg(windows)]
    let path = {
        use std::os::windows::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_wide(&[67, 58, 92, 0xD800]))
    };
    #[cfg(not(windows))]
    let path = {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xFF]))
    };

    assert_eq!(decode_path(&encode_path(&path)).unwrap(), path);
}
