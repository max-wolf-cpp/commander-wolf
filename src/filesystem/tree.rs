use std::fs;
use std::path::Path;

use crate::directory_tree::TreeEntry;

pub fn read_children(directory: &Path, show_hidden: bool) -> Result<Vec<TreeEntry>, String> {
    read_children_bounded(directory, show_hidden, 10_000, &|| false)
}

pub fn expand_all(root: &Path, show_hidden: bool) -> Result<Vec<TreeEntry>, String> {
    expand_all_bounded(root, show_hidden, 10_000, &|| false)
}

pub(crate) fn expand_all_bounded(
    root: &Path,
    show_hidden: bool,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<TreeEntry>, String> {
    let mut entries = Vec::new();
    let mut remaining = limit;
    visit(
        root,
        1,
        show_hidden,
        &mut entries,
        &mut remaining,
        cancelled,
    )?;

    Ok(entries)
}

fn read_children_bounded(
    directory: &Path,
    show_hidden: bool,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<TreeEntry>, String> {
    if cancelled() {
        return Err("Tree expansion cancelled".into());
    }

    let iterator = fs::read_dir(directory).map_err(|error| {
        format!(
            "Cannot read tree directory '{}': {error}",
            directory.display()
        )
    })?;
    let mut entries = Vec::new();
    for (index, item) in iterator.enumerate() {
        if cancelled() {
            return Err("Tree expansion cancelled".into());
        }

        if index >= limit {
            return Err("Tree expansion exceeds entry limit".into());
        }

        let item =
            item.map_err(|error| format!("Cannot enumerate '{}': {error}", directory.display()))?;
        let metadata = fs::symlink_metadata(item.path())
            .map_err(|error| format!("Cannot inspect '{}': {error}", item.path().display()))?;
        entries.push(TreeEntry {
            is_hidden: super::directory::is_hidden(&item.file_name(), &metadata),
            path: item.path(),
            name: item.file_name().to_string_lossy().into_owned(),
            depth: 0,
            is_directory: metadata.is_dir() && !super::is_link(&metadata),
            expanded: false,
            loading: false,
        });
    }

    entries.retain(|entry| show_hidden || !entry.is_hidden);
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

fn visit(
    path: &Path,
    depth: usize,
    show_hidden: bool,
    output: &mut Vec<TreeEntry>,
    remaining: &mut usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if depth > 128 {
        return Err("Tree expansion exceeds depth limit".into());
    }

    let children = read_children_bounded(path, show_hidden, *remaining, cancelled)?;
    *remaining -= children.len();
    for mut entry in children {
        if cancelled() {
            return Err("Tree expansion cancelled".into());
        }

        entry.depth = depth;
        entry.expanded = entry.is_directory;
        let child_path = entry.path.clone();
        let directory = entry.is_directory;
        output.push(entry);
        if directory {
            visit(
                &child_path,
                depth + 1,
                show_hidden,
                output,
                remaining,
                cancelled,
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn tree_expansion_is_bounded_and_cooperatively_cancelled() {
        let temp = TempDirectory::new();
        for name in ["a", "b", "c"] {
            fs::create_dir(temp.join(name)).unwrap();
            fs::write(temp.join(name).join("leaf"), name).unwrap();
        }

        assert!(
            expand_all_bounded(&temp.0, true, 5, &|| false)
                .unwrap_err()
                .contains("limit")
        );

        let entries = expand_all_bounded(&temp.0, true, 6, &|| false).unwrap();
        assert_eq!(
            entries.iter().map(|entry| entry.depth).collect::<Vec<_>>(),
            [1, 2, 1, 2, 1, 2]
        );

        let checks = std::cell::Cell::new(0);
        assert!(
            expand_all_bounded(&temp.0, true, 6, &|| {
                checks.set(checks.get() + 1);
                checks.get() >= 4
            })
            .unwrap_err()
            .contains("cancelled")
        );
        assert!(checks.get() <= 5);
    }
}
