use std::fs;
use std::path::{Path, PathBuf};

use crate::app::Action;
use crate::file_operations::{PanelRefreshRequest, PanelRefreshResult};
use crate::workspace::{DirectoryEntry, DirectorySnapshot, PanelId};

pub fn load_directory(
    panel: PanelId,
    requested_path: &Path,
    preferred: Option<&Path>,
) -> Result<Action, String> {
    load_snapshot(requested_path, preferred, 0).map(|snapshot| Action::DirectoryLoaded {
        panel,
        directory: snapshot.directory,
        entries: snapshot.entries,
        selected: snapshot.selected,
    })
}

pub fn load_snapshot(
    requested_path: &Path,
    preferred: Option<&Path>,
    fallback_index: usize,
) -> Result<DirectorySnapshot, String> {
    let absolute = absolute_normalized(requested_path)?;
    let (directory, startup_file) = if absolute.is_file() {
        let parent = absolute
            .parent()
            .ok_or_else(|| format!("cannot find parent of '{}'", absolute.display()))?;
        (parent.to_path_buf(), Some(absolute.as_path()))
    } else {
        (absolute, None)
    };

    let mut entries = read_entries(&directory)?;
    let parent = directory.parent().filter(|parent| *parent != directory);
    if let Some(parent) = parent {
        entries.insert(
            0,
            DirectoryEntry {
                path: parent.to_path_buf(),
                name: "..".to_owned(),
                is_directory: true,
                is_parent: true,
                is_hidden: false,
            },
        );
    }

    let selected = startup_file
        .or(preferred)
        .and_then(|path| entries.iter().position(|entry| entry.path == path))
        .unwrap_or_else(|| fallback_index.min(entries.len().saturating_sub(1)));

    Ok(DirectorySnapshot {
        directory,
        entries,
        selected,
    })
}

pub fn refresh_panels(requests: [PanelRefreshRequest; 2]) -> [PanelRefreshResult; 2] {
    requests.map(|request| {
        let result = load_snapshot(
            &request.directory,
            request.selection.preferred.as_deref(),
            request.selection.fallback_index,
        );
        PanelRefreshResult { request, result }
    })
}

pub fn reveal_finder_result(
    generation: u64,
    panel: PanelId,
    path: &Path,
) -> Result<Action, String> {
    fs::symlink_metadata(path)
        .map_err(|error| format!("Finder result '{}' is unavailable: {error}", path.display()))?;
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Finder result '{}' has no containing directory",
            path.display()
        )
    })?;

    let snapshot = load_snapshot(parent, Some(path), 0)?;
    if snapshot
        .entries
        .get(snapshot.selected)
        .is_none_or(|entry| entry.path != path)
    {
        return Err(format!(
            "Finder result '{}' disappeared before it could be revealed",
            path.display()
        ));
    }

    Ok(Action::FinderRevealLoaded {
        generation,
        panel,
        directory: snapshot.directory,
        entries: snapshot.entries,
        selected: snapshot.selected,
    })
}

pub(super) fn read_entries(directory: &Path) -> Result<Vec<DirectoryEntry>, String> {
    let iterator = fs::read_dir(directory)
        .map_err(|error| format!("cannot read directory '{}': {error}", directory.display()))?;
    let mut entries = Vec::new();
    for item in iterator {
        let item = item.map_err(|error| {
            format!(
                "cannot enumerate directory '{}': {error}",
                directory.display()
            )
        })?;
        let file_type = item.file_type().map_err(|error| {
            format!("cannot inspect entry '{}': {error}", item.path().display())
        })?;
        let metadata = item.metadata().map_err(|error| {
            format!("cannot inspect entry '{}': {error}", item.path().display())
        })?;

        entries.push(DirectoryEntry {
            path: item.path(),
            name: item.file_name().to_string_lossy().into_owned(),
            is_directory: file_type.is_dir(),
            is_parent: false,
            is_hidden: is_hidden(&item.file_name(), &metadata),
        });
    }

    entries.sort_by(|left, right| {
        right.is_directory.cmp(&left.is_directory).then_with(|| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        })
    });

    Ok(entries)
}

fn absolute_normalized(path: &Path) -> Result<PathBuf, String> {
    let base = std::env::current_dir()
        .map_err(|error| format!("cannot resolve current directory: {error}"))?;
    Ok(crate::file_operations::normalize_from(&base, path))
}

pub(super) fn is_hidden(name: &std::ffi::OsStr, metadata: &fs::Metadata) -> bool {
    let dot_hidden = name.to_string_lossy().starts_with('.');
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN_OR_SYSTEM: u32 = 0x2 | 0x4;
        dot_hidden || metadata.file_attributes() & HIDDEN_OR_SYSTEM != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        dot_hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory as TestDirectory;

    #[test]
    fn hidden_entries_are_classified_and_filtered_consistently() {
        let root = TestDirectory::new();
        fs::write(root.0.join(".secret"), "hidden").unwrap();
        fs::create_dir(root.0.join(".private")).unwrap();
        fs::write(root.0.join(".private/visible-name"), "hidden subtree").unwrap();
        fs::write(root.0.join("visible"), "shown").unwrap();

        let snapshot = load_snapshot(&root.0, None, 0).unwrap();
        assert_eq!(
            snapshot
                .entries
                .iter()
                .filter(|entry| entry.is_hidden)
                .count(),
            2
        );
        assert!(!snapshot.entries[0].is_hidden);

        let tree = crate::filesystem::tree::expand_all(&root.0, false).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name, "visible");

        let preview = crate::filesystem::directory_preview::load(&root.0, false).unwrap();
        assert_eq!(preview.entries.len(), 1);
        assert_eq!(preview.entries[0].name, "visible");
    }

    #[cfg(windows)]
    #[test]
    fn windows_hidden_and_system_attributes_are_classified() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = TestDirectory::new();
        for (name, attributes) in [("hidden", 0x2), ("system", 0x4), ("both", 0x6)] {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .attributes(attributes)
                .open(root.0.join(name))
                .unwrap();
        }

        let entries = read_entries(&root.0).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().all(|entry| entry.is_hidden));
    }

    #[test]
    fn adds_parent_and_sorts_directories_before_unicode_files() {
        let root = TestDirectory::new();
        fs::create_dir(root.0.join("目录")).unwrap();
        fs::write(root.0.join("狼.txt"), "wolf").unwrap();
        fs::write(root.0.join("Alpha.txt"), "alpha").unwrap();

        let snapshot = load_snapshot(&root.0, None, 0).unwrap();
        assert!(snapshot.directory.is_absolute());
        assert_eq!(snapshot.entries[0].name, "..");
        assert_eq!(snapshot.entries[1].name, "目录");
        assert_eq!(snapshot.entries[2].name, "Alpha.txt");
        assert_eq!(snapshot.entries[3].name, "狼.txt");
    }

    #[test]
    fn file_path_selects_file_and_refresh_uses_fallback() {
        let root = TestDirectory::new();
        let file = root.0.join("目标 file.txt");
        fs::write(&file, "target").unwrap();
        let snapshot = load_snapshot(&file, None, 0).unwrap();
        assert_eq!(snapshot.entries[snapshot.selected].path, file);

        let snapshot = load_snapshot(&root.0, Some(Path::new("missing")), 1).unwrap();
        assert_eq!(snapshot.selected, 1);
    }

    #[test]
    fn refresh_after_external_rename_delete_and_directory_removal_is_explicit() {
        let root = TestDirectory::new();
        let current = root.0.join("current");
        fs::create_dir(&current).unwrap();
        let selected = current.join("selected");
        fs::write(&selected, "contents").unwrap();
        let before = load_snapshot(&current, Some(&selected), usize::MAX).unwrap();
        assert_eq!(before.entries[before.selected].path, selected);

        let renamed = current.join("renamed");
        fs::rename(&selected, &renamed).unwrap();
        let after = load_snapshot(&current, Some(&selected), before.selected).unwrap();
        assert_eq!(after.entries[after.selected].path, renamed);

        fs::remove_file(&renamed).unwrap();
        let empty = load_snapshot(&current, Some(&renamed), usize::MAX).unwrap();
        assert!(empty.selected < empty.entries.len());
        assert!(empty.entries[empty.selected].is_parent);

        fs::remove_dir(&current).unwrap();
        assert!(load_snapshot(&current, None, 0).is_err());
        assert!(load_snapshot(&root.0, None, 0).is_ok());
    }

    #[test]
    fn panel_refresh_attempts_both_directories_independently() {
        let root = TestDirectory::new();
        let missing = root.0.join("missing");
        let requests = [
            PanelRefreshRequest {
                panel: PanelId::Left,
                directory: missing,
                selection: crate::file_operations::RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            },
            PanelRefreshRequest {
                panel: PanelId::Right,
                directory: root.0.clone(),
                selection: crate::file_operations::RefreshSelection {
                    preferred: None,
                    fallback_index: 0,
                },
            },
        ];

        let results = refresh_panels(requests);
        assert!(results[0].result.is_err());
        assert!(results[1].result.is_ok());
    }

    #[test]
    fn finder_reveal_selects_files_and_directories_in_their_parent() {
        let root = TestDirectory::new();
        let directory = root.0.join("目录");
        let file = root.0.join("file.txt");
        fs::create_dir(&directory).unwrap();
        fs::write(&file, "x").unwrap();

        for path in [&directory, &file] {
            let Action::FinderRevealLoaded {
                panel,
                directory: loaded,
                entries,
                selected,
                ..
            } = reveal_finder_result(7, PanelId::Right, path).unwrap()
            else {
                panic!("typed finder reveal action expected")
            };

            assert_eq!(panel, PanelId::Right);
            assert_eq!(loaded, root.0);
            assert_eq!(entries[selected].path, *path);
        }

        assert!(
            reveal_finder_result(7, PanelId::Left, &root.0.join("missing"))
                .unwrap_err()
                .contains("unavailable")
        );
    }
}
