use super::*;

#[test]
fn captures_both_streams_cwd_and_unsuccessful_exit_without_a_system_shell() {
    let mut runtime = ShellRuntime::default();
    let directory: std::path::PathBuf = std::env::temp_dir().components().collect();
    assert!(
        runtime
            .start("shell-fixture", &directory, Some(fixture_shell()))
            .is_empty()
    );

    let actions = wait_for_finish(&mut runtime);
    let output = actions
        .iter()
        .filter_map(|action| match action {
            Action::ShellOutput(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();

    assert!(output.contains("stdout 狼"), "{output}");
    assert!(output.contains("stderr 狼"), "{output}");
    assert!(
        output.contains(&format!("cwd={}", directory.display())),
        "{output}"
    );
    assert!(actions.contains(&Action::ShellFinished {
        message: "Exit code: 7".into(),
        failed: true
    }));
}

#[test]
fn reports_launch_errors_and_cancels_a_running_child() {
    let mut runtime = ShellRuntime::default();
    let directory = std::env::temp_dir();
    let missing = Application {
        program: directory
            .join("nonexistent-commander-wolf-shell-executable")
            .to_string_lossy()
            .into_owned(),
        arguments: vec![],
    };
    assert!(matches!(
        runtime
            .start("anything", &directory, Some(missing))
            .as_slice(),
        [Action::ShellFinished { failed: true, .. }]
    ));

    assert!(
        runtime
            .start("shell-fixture-wait", &directory, Some(fixture_shell()))
            .is_empty()
    );
    assert!(runtime.cancel().is_empty());

    let actions = wait_for_finish(&mut runtime);
    assert!(actions.contains(&Action::ShellFinished {
        message: "Command interrupted".into(),
        failed: true
    }));
}

#[test]
fn process_tree_cancellation_parent_exit_and_drop_release_inherited_pipes() {
    use std::io::Write;
    for mode in ["cancel", "parent-exit", "drop"] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut runtime = ShellRuntime::default();
        assert!(
            runtime
                .start(
                    &format!("shell-fixture-tree:{}", listener.local_addr().unwrap()),
                    &std::env::temp_dir(),
                    Some(fixture_shell())
                )
                .is_empty()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut parent = None;
        let mut descendant = None;
        while parent.is_none() || descendant.is_none() {
            match listener.accept() {
                Ok((mut connection, _)) => {
                    connection.set_nonblocking(false).unwrap();
                    connection
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut role = [0];
                    connection.read_exact(&mut role).unwrap();
                    if role == *b"P" {
                        parent = Some(connection);
                    } else {
                        descendant = Some(connection);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "fixture did not connect");
                    thread::yield_now();
                }
                Err(error) => panic!("{error}"),
            }
        }

        match mode {
            "cancel" => {
                assert!(runtime.cancel().is_empty());
                let actions = wait_for_finish(&mut runtime);
                assert_eq!(
                    actions
                        .iter()
                        .filter(|action| matches!(action, Action::ShellFinished { .. }))
                        .count(),
                    1
                );
            }
            "parent-exit" => {
                parent.as_mut().unwrap().write_all(b"x").unwrap();
                wait_for_finish(&mut runtime);
            }
            _ => runtime.shutdown(),
        }

        let result = descendant.as_mut().unwrap().read(&mut [0]);
        assert!(
            matches!(result, Ok(0))
                || result.as_ref().is_err_and(|error| matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                )),
            "descendant remained alive: {result:?}"
        );
        assert!(runtime.running.is_none());
    }
}

#[test]
fn concurrent_large_streams_are_drained_with_bounded_event_loop_work() {
    let mut runtime = ShellRuntime::default();
    assert!(
        runtime
            .start(
                "shell-fixture-flood",
                &std::env::temp_dir(),
                Some(fixture_shell())
            )
            .is_empty()
    );

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stdout = 0;
    let mut stderr = 0;
    let mut terminal = 0;
    let mut retained = crate::shell::ShellOutput::new("fixture".into());
    while runtime.running.is_some() {
        let actions = runtime.poll();
        assert!(actions.len() <= 65);
        for action in actions {
            match action {
                Action::ShellOutput(text) => {
                    stdout += text.bytes().filter(|&b| b == b'o').count();
                    stderr += text.bytes().filter(|&b| b == b'e').count();
                    retained.append(&text);
                }
                Action::ShellFinished { failed, .. } => {
                    assert!(!failed);
                    terminal += 1;
                }
                _ => panic!("unexpected action"),
            }
        }
        assert!(Instant::now() < deadline, "output readers stalled");
        thread::yield_now();
    }

    assert!(stdout >= 1024 * 1024 && stderr >= 1024 * 1024);
    assert_eq!(terminal, 1);
    assert!(retained.truncated);
    assert!(retained.lines.iter().map(String::len).sum::<usize>() <= 1024 * 1024);
}

#[test]
#[ignore = "requires the platform's default shell"]
fn default_shell_smoke() {
    let mut runtime = ShellRuntime::default();
    #[cfg(windows)]
    let source =
        "Write-Output 'wolf-shell-ok 狼'; [Console]::Error.WriteLine('wolf-shell-error'); exit 3";
    #[cfg(not(windows))]
    let source = "printf 'wolf-shell-ok 狼\\n'; printf 'wolf-shell-error\\n' >&2; exit 3";
    assert!(
        runtime
            .start(source, &std::env::temp_dir(), None)
            .is_empty()
    );

    let actions = wait_for_finish(&mut runtime);
    let text = actions
        .iter()
        .filter_map(|action| match action {
            Action::ShellOutput(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains("wolf-shell-ok 狼"), "{text}");
    assert!(text.contains("wolf-shell-error"), "{text}");
    assert!(actions.contains(&Action::ShellFinished {
        message: "Exit code: 3".into(),
        failed: true
    }));
}
