use std::path::Path;
use std::process::{Command, Stdio};

use super::Application;

pub fn open_external(path: &Path, application: Option<&Application>) -> Result<(), String> {
    match application {
        None => open::that_detached(path),
        Some(application) if application.arguments.is_empty() => {
            open::with_detached(path, &application.program)
        }
        Some(application) => spawn_with_arguments(path, application),
    }
    .map_err(|error| {
        let target = application.map_or("the operating system default".to_owned(), |application| {
            format!("'{}'", application.program)
        });

        format!("Cannot open '{}' with {target}: {error}", path.display())
    })
}

pub fn run_text_editor(path: &Path, editor: &Application) -> Result<(), String> {
    let status = Command::new(&editor.program)
        .args(&editor.arguments)
        .arg(path)
        .status()
        .map_err(|error| {
            format!(
                "Cannot open '{}' with text editor '{}': {error}",
                path.display(),
                editor.program
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Text editor '{}' exited unsuccessfully while opening '{}'",
            editor.program,
            path.display()
        ))
    }
}

pub fn run_editor_in_directory(directory: &Path, editor: &Application) -> Result<(), String> {
    let status = Command::new(&editor.program)
        .args(&editor.arguments)
        .arg(".")
        .current_dir(directory)
        .status()
        .map_err(|error| {
            format!(
                "Cannot open directory '{}' with text editor '{}': {error}",
                directory.display(),
                editor.program
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Text editor '{}' exited unsuccessfully while opening directory '{}'",
            editor.program,
            directory.display()
        ))
    }
}

fn spawn_with_arguments(path: &Path, application: &Application) -> std::io::Result<()> {
    let mut command = Command::new(&application.program);
    command
        .args(&application.arguments)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    spawn_detached(&mut command)
}

pub(super) fn spawn_detached(command: &mut Command) -> std::io::Result<()> {
    #[cfg(windows)]
    configure_detached_process(command);

    let child = command.spawn()?;
    #[cfg(unix)]
    {
        let mut child = child;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    #[cfg(not(unix))]
    drop(child);

    Ok(())
}

#[cfg(windows)]
fn configure_detached_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}
