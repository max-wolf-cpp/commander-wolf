use std::path::{Component, Path, PathBuf};

pub fn normalize_from(base: &Path, path: &Path) -> PathBuf {
    let mut normalized = if path.is_absolute() {
        PathBuf::new()
    } else {
        base.to_path_buf()
    };

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(
                    normalized.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    normalized.pop();
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    normalized
}

pub fn validate_new_name(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("rename usage: :rename <new-name>; new name cannot be empty".into());
    }

    let path = Path::new(value);
    if value == "."
        || value == ".."
        || path.is_absolute()
        || value.contains('/')
        || value.contains('\\')
        || path.components().count() != 1
    {
        return Err(format!(
            "rename usage: :rename <new-name>; '{value}' must be one file or directory name"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_accepts_unicode_spaces_and_rejects_paths() {
        assert!(validate_new_name("новое имя.txt").is_ok());

        for invalid in ["", ".", "..", "a/b", "a\\b"] {
            assert!(validate_new_name(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn relative_paths_are_normalized_against_panel_directory() {
        let base = Path::new("/work/panel");

        assert_eq!(
            normalize_from(base, Path::new("../目标 file")),
            Path::new("/work/目标 file")
        );
    }
}
