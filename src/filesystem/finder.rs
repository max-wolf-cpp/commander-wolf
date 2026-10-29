use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::finder::{FinderEntryKind, FinderOptions, FinderResult, GlobMatcher};

const BATCH_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FinderWalkOutcome {
    pub skipped: usize,
    pub truncated: bool,
    pub cancelled: bool,
}

pub fn walk(
    root: &Path,
    matcher: &GlobMatcher,
    options: FinderOptions,
    cancelled: &AtomicBool,
    mut publish: impl FnMut(Vec<FinderResult>) -> bool,
) -> Result<FinderWalkOutcome, String> {
    let root = resolve_root(root)?;

    let mut outcome = FinderWalkOutcome::default();
    let mut directories = vec![root.clone()];
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut matched = 0usize;
    while let Some(directory) = directories.pop() {
        if cancelled.load(Ordering::Relaxed) {
            outcome.cancelled = true;
            break;
        }

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) if directory != root => {
                outcome.skipped += 1;
                continue;
            }
            Err(error) => {
                return Err(format!(
                    "Unable to read finder root '{}': {error}",
                    root.display()
                ));
            }
        };
        for entry in entries {
            if cancelled.load(Ordering::Relaxed) {
                outcome.cancelled = true;
                break;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    outcome.skipped += 1;
                    continue;
                }
            };

            let path = entry.path();
            let Some((kind, is_hidden)) =
                classify_entry(&path, &entry.file_name(), options.show_hidden, &mut outcome)
            else {
                continue;
            };

            let relative = match path.strip_prefix(&root) {
                Ok(relative) => relative,
                Err(_) => {
                    outcome.skipped += 1;
                    continue;
                }
            };

            let relative_path = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let basename = entry.file_name().to_string_lossy().into_owned();
            if matcher.matches(
                &relative_path,
                &basename,
                kind == FinderEntryKind::Directory,
            ) {
                if matched == options.max_results {
                    outcome.truncated = true;
                    break;
                }

                batch.push(FinderResult {
                    is_hidden,
                    absolute_path: path.clone(),
                    relative_path,
                    kind,
                });
                matched += 1;
                if batch.len() == BATCH_SIZE && !publish(std::mem::take(&mut batch)) {
                    outcome.cancelled = true;
                    break;
                }
            }

            if kind == FinderEntryKind::Directory {
                directories.push(path);
            }
        }

        if outcome.cancelled || outcome.truncated {
            break;
        }
    }

    if !batch.is_empty() && !publish(batch) {
        outcome.cancelled = true;
    }

    Ok(outcome)
}

fn resolve_root(root: &Path) -> Result<PathBuf, String> {
    let base = std::env::current_dir()
        .map_err(|error| format!("Unable to resolve the finder root: {error}"))?;
    let root = crate::file_operations::normalize_from(&base, root);
    let root_metadata = fs::metadata(&root).map_err(|error| {
        format!(
            "Unable to inspect finder root '{}': {error}",
            root.display()
        )
    })?;
    if !root_metadata.is_dir() {
        return Err(format!(
            "Finder root is not a directory: '{}'",
            root.display()
        ));
    }

    fs::read_dir(&root)
        .map_err(|error| format!("Unable to read finder root '{}': {error}", root.display()))?;

    Ok(root)
}

fn classify_entry(
    path: &Path,
    name: &OsStr,
    show_hidden: bool,
    outcome: &mut FinderWalkOutcome,
) -> Option<(FinderEntryKind, bool)> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            outcome.skipped += 1;
            return None;
        }
    };
    if super::is_link(&metadata) {
        outcome.skipped += 1;
        return None;
    }

    let is_hidden = super::directory::is_hidden(name, &metadata);
    if !show_hidden && is_hidden {
        return None;
    }

    let kind = if metadata.is_dir() {
        FinderEntryKind::Directory
    } else if metadata.is_file() {
        FinderEntryKind::File
    } else {
        outcome.skipped += 1;
        return None;
    };

    Some((kind, is_hidden))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "commander-wolf-finder-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();

            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn collect(root: &Path, pattern: &str, show_hidden: bool, max: usize) -> Vec<FinderResult> {
        let mut results = Vec::new();
        walk(
            root,
            &GlobMatcher::parse(pattern).unwrap(),
            FinderOptions {
                show_hidden,
                max_results: max,
                ..FinderOptions::default()
            },
            &AtomicBool::new(false),
            |batch| {
                results.extend(batch);
                true
            },
        )
        .unwrap();
        results.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        results
    }

    #[test]
    fn recursively_streams_files_and_directories_and_prunes_hidden_trees() {
        let root = TestDirectory::new();
        fs::create_dir_all(root.0.join("Source/Nested")).unwrap();
        fs::create_dir_all(root.0.join(".git")).unwrap();
        fs::write(root.0.join("Source/Nested/Command.cpp"), "x").unwrap();
        fs::write(root.0.join(".git/hidden.cpp"), "x").unwrap();
        fs::write(root.0.join(".secret.cpp"), "x").unwrap();

        let visible = collect(&root.0, "*.cpp", false, 100);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].relative_path, "Source/Nested/Command.cpp");
        assert_eq!(collect(&root.0, "*.cpp", true, 100).len(), 3);
        assert!(
            collect(&root.0, "*.cpp", true, 100)
                .iter()
                .find(|entry| entry.relative_path == ".secret.cpp")
                .unwrap()
                .is_hidden
        );
        assert_eq!(collect(&root.0, "*/Nested/", false, 100).len(), 1);
    }

    #[test]
    fn cancellation_and_result_limit_stop_without_discarding_batches() {
        let root = TestDirectory::new();
        for index in 0..20 {
            fs::write(root.0.join(format!("{index}.txt")), "x").unwrap();
        }

        let cancellation = AtomicBool::new(false);
        let mut published = 0;
        let outcome = walk(
            &root.0,
            &GlobMatcher::parse("*.txt").unwrap(),
            FinderOptions {
                max_results: 3,
                ..FinderOptions::default()
            },
            &cancellation,
            |batch| {
                published += batch.len();
                true
            },
        )
        .unwrap();

        assert_eq!(published, 3);
        assert!(outcome.truncated);
    }

    #[test]
    fn missing_root_reports_an_error() {
        let root = TestDirectory::new();
        let error = walk(
            &root.0.join("missing"),
            &GlobMatcher::parse("*").unwrap(),
            FinderOptions::default(),
            &AtomicBool::new(false),
            |_| true,
        )
        .unwrap_err();
        assert!(error.contains("finder root"));
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "requires Windows Developer Mode or symlink privilege"
    )]
    fn platform_directory_symlinks_are_never_traversed() {
        let root = TestDirectory::new();
        let target = root.0.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inside.txt"), "x").unwrap();
        let link = root.0.join("link");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&target, &link).is_ok();
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&target, &link).is_ok();
        assert!(
            linked,
            "environment requires directory symlink creation privilege"
        );

        let results = collect(&root.0, "link/**", true, 100);
        assert!(results.is_empty());
    }
}
