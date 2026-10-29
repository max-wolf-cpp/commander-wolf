use std::path::Path;

#[derive(Default)]
pub(super) struct ClipboardRuntime {
    // X11 and Wayland serve pasted text from the process that owns the clipboard.
    clipboard: Option<arboard::Clipboard>,
}

impl ClipboardRuntime {
    pub fn copy_path(&mut self, path: &Path) -> Result<(), String> {
        self.copy_text(&full_path_text(path)?)
    }

    pub fn copy_name(&mut self, path: &Path) -> Result<(), String> {
        self.copy_text(file_name_text(path)?)
    }

    fn copy_text(&mut self, text: &str) -> Result<(), String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|error| format!("System clipboard unavailable: {error}"))?,
            );
        }

        self.clipboard
            .as_mut()
            .expect("clipboard initialized above")
            .set_text(text)
            .map_err(|error| format!("Cannot copy text to the system clipboard: {error}"))
    }
}

fn full_path_text(path: &Path) -> Result<String, String> {
    // Preserve the selected symlink path rather than resolving its target.
    let absolute = std::path::absolute(path)
        .map_err(|error| format!("Cannot resolve the selected path: {error}"))?;

    absolute
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| "The selected path cannot be represented as Unicode text".to_owned())
}

fn file_name_text(path: &Path) -> Result<&str, String> {
    path.file_name()
        .ok_or_else(|| "The selected path has no file name".to_owned())?
        .to_str()
        .ok_or_else(|| "The selected name cannot be represented as Unicode text".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_paths_are_absolute_without_requiring_the_entry_to_exist() {
        let relative = Path::new("clipboard test directory").join("новый файл.tar.gz");
        let expected = std::env::current_dir().unwrap().join(&relative);
        assert_eq!(
            full_path_text(&relative).unwrap(),
            expected.to_str().unwrap()
        );
        assert_eq!(
            full_path_text(&expected).unwrap(),
            expected.to_str().unwrap()
        );
    }

    #[test]
    fn names_preserve_extensions_spaces_and_unicode() {
        for name in [
            "новый файл.tar.gz",
            ".gitignore",
            "folder with spaces",
            "目录",
        ] {
            let path = Path::new("parent").join(name);
            assert_eq!(file_name_text(&path).unwrap(), name);
        }
    }

    #[test]
    fn empty_path_has_no_name() {
        assert!(file_name_text(Path::new("")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_names_fail_without_replacing_bytes() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        assert!(file_name_text(Path::new(OsStr::from_bytes(b"file\xff.txt"))).is_err());
        assert!(full_path_text(Path::new(OsStr::from_bytes(b"file\xff.txt"))).is_err());
    }
}
