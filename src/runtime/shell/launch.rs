use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::file_opening::Application;

pub(super) fn spawn(
    source: &str,
    directory: &Path,
    shell: Option<Application>,
) -> io::Result<(Child, super::super::process_tree::ProcessTree)> {
    #[cfg(windows)]
    let automatic_source = format!(
        "$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); if (Get-Variable PSStyle -ErrorAction Ignore) {{ $PSStyle.OutputRendering = 'PlainText' }}; {source}"
    );
    #[cfg(windows)]
    let source = if shell.is_none() {
        automatic_source.as_str()
    } else {
        source
    };

    let candidates = shell
        .map(|shell| vec![shell])
        .unwrap_or_else(default_shells);
    let mut error = io::Error::new(io::ErrorKind::NotFound, "no command shell available");
    for shell in candidates {
        let mut command = Command::new(&shell.program);
        command
            .args(&shell.arguments)
            .arg(source)
            .current_dir(directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("NO_COLOR", "1")
            .env("TERM", "dumb");

        match super::super::process_tree::spawn(&mut command) {
            Ok(child) => return Ok(child),
            Err(next) if next.kind() == io::ErrorKind::NotFound => error = next,
            Err(error) => return Err(error),
        }
    }

    Err(error)
}

#[cfg(windows)]
fn default_shells() -> Vec<Application> {
    ["pwsh", "powershell"]
        .into_iter()
        .map(|program| Application {
            program: program.into(),
            arguments: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
            ],
        })
        .collect()
}

#[cfg(not(windows))]
fn default_shells() -> Vec<Application> {
    let program = std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/sh".into());

    vec![
        Application {
            program,
            arguments: vec!["-c".into()],
        },
        Application {
            program: "/bin/sh".into(),
            arguments: vec!["-c".into()],
        },
    ]
}
