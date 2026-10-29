#[cfg(windows)]
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use super::process::spawn_detached;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileManagerInvocation {
    program: &'static str,
    arguments: Vec<OsString>,
}

pub fn reveal_in_file_manager(path: &Path, is_directory: bool) -> Result<(), String> {
    launch_file_manager(file_manager_invocation(path, is_directory), path)
}

fn launch_file_manager(invocation: FileManagerInvocation, path: &Path) -> Result<(), String> {
    let mut command = Command::new(invocation.program);
    #[cfg(windows)]
    if let [switch, target] = invocation.arguments.as_slice()
        && switch == "/select,"
    {
        use std::os::windows::process::CommandExt;

        command.arg(switch).raw_arg(quoted_explorer_path(target)?);
    } else {
        command.args(&invocation.arguments);
    }
    #[cfg(not(windows))]
    command.args(&invocation.arguments);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    spawn_detached(&mut command).map_err(|error| {
        format!(
            "Cannot reveal '{}' with system file manager '{}': {error}",
            path.display(),
            invocation.program
        )
    })
}

#[cfg(windows)]
fn quoted_explorer_path(path: &OsStr) -> Result<OsString, String> {
    if path.as_encoded_bytes().contains(&b'"') {
        return Err("Cannot reveal a Windows path containing a double quote".into());
    }

    // Explorer uses commas as separators even when Rust would not quote the path.
    // Windows file names cannot contain double quotes, so no embedded quote can
    // terminate this raw argument and introduce another Explorer option.
    let mut quoted = OsString::from("\"");
    quoted.push(path);
    quoted.push("\"");

    Ok(quoted)
}

#[cfg(windows)]
fn file_manager_invocation(path: &Path, is_directory: bool) -> FileManagerInvocation {
    let arguments = if is_directory {
        vec![path.as_os_str().to_owned()]
    } else {
        // Explorer must see the switch outside the quotes around a spaced path.
        vec![OsString::from("/select,"), path.as_os_str().to_owned()]
    };

    FileManagerInvocation {
        program: "explorer.exe",
        arguments,
    }
}

#[cfg(target_os = "macos")]
fn file_manager_invocation(path: &Path, is_directory: bool) -> FileManagerInvocation {
    let mut arguments = Vec::with_capacity(2);
    if !is_directory {
        arguments.push(OsString::from("-R"));
    }
    arguments.push(path.as_os_str().to_owned());

    FileManagerInvocation {
        program: "open",
        arguments,
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn file_manager_invocation(path: &Path, is_directory: bool) -> FileManagerInvocation {
    let target = if is_directory {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    FileManagerInvocation {
        program: "xdg-open",
        arguments: vec![target.as_os_str().to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_manager_invocation_opens_directories_and_reveals_files() {
        let directory = Path::new("some directory");
        let file = directory.join("wolf.txt");
        let directory_invocation = file_manager_invocation(directory, true);
        let file_invocation = file_manager_invocation(&file, false);

        #[cfg(windows)]
        {
            assert_eq!(directory_invocation.program, "explorer.exe");
            assert_eq!(directory_invocation.arguments, [directory.as_os_str()]);
            assert_eq!(
                file_invocation.arguments,
                [OsString::from("/select,"), file.into_os_string()]
            );
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(directory_invocation.program, "open");
            assert_eq!(directory_invocation.arguments, [directory.as_os_str()]);
            assert_eq!(
                file_invocation.arguments,
                [OsString::from("-R"), file.into_os_string()]
            );
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!(directory_invocation.program, "xdg-open");
            assert_eq!(directory_invocation.arguments, [directory.as_os_str()]);
            assert_eq!(file_invocation.arguments, [directory.as_os_str()]);
        }
    }

    #[cfg(windows)]
    #[test]
    fn explorer_selection_keeps_the_switch_separate_from_special_paths() {
        for path in [
            r"C:\files\00 - Inbox\Папка с пробелами\Тест, пример\sample.rar",
            r"C:\folder with spaces\image.png",
            r"C:\plain\кириллица.txt",
            r"C:\plain\comma,name.txt",
            r"C:\plain\file.txt",
            r"\\server\shared folder\file name.txt",
        ] {
            let invocation = file_manager_invocation(Path::new(path), false);
            assert_eq!(
                invocation.arguments,
                [OsString::from("/select,"), OsString::from(path)]
            );
            assert_eq!(
                quoted_explorer_path(OsStr::new(path)).unwrap(),
                OsString::from(format!("\"{path}\""))
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn explorer_selection_rejects_embedded_quotes_before_launching() {
        assert!(
            reveal_in_file_manager(Path::new(r#"C:\file",/root,C:\Windows"#), false)
                .unwrap_err()
                .contains("double quote")
        );
    }

    #[test]
    fn missing_file_manager_returns_a_visible_diagnostic() {
        let path = Path::new("missing/wolf.txt");
        let error = launch_file_manager(
            FileManagerInvocation {
                program: "commander-wolf-file-manager-that-does-not-exist",
                arguments: Vec::new(),
            },
            path,
        )
        .unwrap_err();

        assert!(error.contains("commander-wolf-file-manager-that-does-not-exist"));
        assert!(error.contains(&path.display().to_string()));
    }
}
