use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct EntryPaths {
    entries: BTreeMap<PathBuf, bool>,
    count: usize,
}

impl EntryPaths {
    pub fn register(&mut self, name: &str, directory: bool) -> Result<PathBuf, String> {
        self.count += 1;
        if self.count > super::MAX_ENTRIES {
            return Err("Archive exceeds the 100,000 entry limit".into());
        }

        let path = relative_path(name)?;
        if path.as_os_str().is_empty() && !directory {
            return Err("Archive file has an empty path".into());
        }

        let key = path_key(&path);
        if let Some(was_directory) = self.entries.get(&key)
            && (!directory || !was_directory)
        {
            return Err(format!("Conflicting archive entry: '{name}'"));
        }

        for parent in key.ancestors().skip(1) {
            if self.entries.get(parent) == Some(&false) {
                return Err(format!("Archive entry has a file as its parent: '{name}'"));
            }
            self.entries.entry(parent.to_owned()).or_insert(true);
        }
        self.entries.insert(key, directory);

        Ok(path)
    }
}

fn path_key(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

fn relative_path(name: &str) -> Result<PathBuf, String> {
    if name.len() > 4096 || name.split(['/', '\\']).count() > 128 {
        return Err("Archive entry path is too long or deeply nested".into());
    }

    if name.starts_with(['/', '\\']) || name.contains(['\0', ':']) {
        return Err(format!("Unsafe archive path: '{name}'"));
    }

    let mut path = PathBuf::new();
    for part in name.split(['/', '\\']) {
        if part == ".." {
            return Err(format!("Unsafe archive path: '{name}'"));
        }

        if part.is_empty() || part == "." {
            continue;
        }

        validate_component(part)?;
        path.push(part);
    }

    Ok(path)
}

fn validate_component(part: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let base = part
            .split('.')
            .next()
            .unwrap_or(part)
            .trim_end_matches([' ', '.'])
            .to_ascii_uppercase();
        let numbered_device = ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        if part.ends_with(['.', ' '])
            || part.contains(['<', '>', '"', '|', '?', '*'])
            || part.chars().any(char::is_control)
            || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || numbered_device
        {
            return Err(format!("Unsafe Windows archive name: '{part}'"));
        }
    }
    let _ = part;
    Ok(())
}
