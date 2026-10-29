use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::*;

mod decoding;
mod fixture;
mod processes;

fn fixture_shell() -> Application {
    Application {
        program: std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        arguments: vec![
            "--exact".into(),
            "runtime::shell::tests::process_fixture".into(),
            "--nocapture".into(),
        ],
    }
}

#[test]
fn process_fixture() {
    fixture::run();
}

fn wait_for_finish(runtime: &mut ShellRuntime) -> Vec<Action> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut actions = Vec::new();
    while runtime.running.is_some() && Instant::now() < deadline {
        actions.extend(runtime.poll());
        thread::sleep(Duration::from_millis(5));
    }

    assert!(runtime.running.is_none(), "child process did not finish");

    actions
}
