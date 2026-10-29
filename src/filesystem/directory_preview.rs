use std::path::Path;

use crate::preview::{DirectoryPreview, DirectoryPreviewEntry};

const MAX_PREVIEW_ENTRIES: usize = 4096;

pub fn load(path: &Path, show_hidden: bool) -> Result<DirectoryPreview, String> {
    let mut entries = super::directory::read_entries(path).map_err(|message| {
        format!(
            "Cannot load directory preview for '{}': {message}",
            path.display()
        )
    })?;

    entries.retain(|entry| show_hidden || !entry.is_hidden);
    let truncated = entries.len() > MAX_PREVIEW_ENTRIES;
    entries.truncate(MAX_PREVIEW_ENTRIES);

    Ok(DirectoryPreview {
        entries: entries
            .into_iter()
            .map(|entry| DirectoryPreviewEntry {
                is_hidden: entry.is_hidden,
                name: entry.name,
                is_directory: entry.is_directory,
            })
            .collect(),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "commander-wolf-directory-preview-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();

            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn lists_immediate_children_with_directories_first_and_handles_empty_directories() {
        let root = TestDirectory::new();
        fs::write(root.0.join("z.txt"), b"Wolf").unwrap();
        fs::create_dir(root.0.join("Alpha")).unwrap();

        let preview = load(&root.0, true).unwrap();
        assert_eq!(preview.entries.len(), 2);
        assert_eq!(preview.entries[0].name, "Alpha");
        assert!(preview.entries[0].is_directory);
        assert_eq!(preview.entries[1].name, "z.txt");
        assert!(!preview.entries[1].is_directory);
        assert!(!preview.truncated);

        assert!(
            load(&root.0.join("Alpha"), true)
                .unwrap()
                .entries
                .is_empty()
        );
    }
}
