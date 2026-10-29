use std::io::Write;

use super::*;

pub(super) fn run() {
    for argument in std::env::args() {
        if let Some(address) = argument.strip_prefix("shell-fixture-descendant:") {
            run_descendant(address);
        }

        if let Some(address) = argument.strip_prefix("shell-fixture-tree:") {
            run_tree_parent(address);
        }

        if argument == "shell-fixture-flood" {
            run_flood();
        }
    }

    if std::env::args().any(|arg| arg == "shell-fixture-wait") {
        println!("ready");
        loop {
            thread::sleep(Duration::from_millis(20));
        }
    }

    if std::env::args().any(|arg| arg == "shell-fixture") {
        println!("stdout 狼");
        eprintln!("stderr 狼");
        println!("cwd={}", std::env::current_dir().unwrap().display());
        std::process::exit(7);
    }
}

fn run_descendant(address: &str) -> ! {
    let mut connection = std::net::TcpStream::connect(address).unwrap();
    connection.write_all(b"D").unwrap();
    println!("descendant stdout");
    eprintln!("descendant stderr");
    let _ = connection.read(&mut [0]);

    std::process::exit(0);
}

fn run_tree_parent(address: &str) -> ! {
    let fixture = fixture_shell();
    let mut child = Command::new(fixture.program)
        .args(fixture.arguments)
        .arg(format!("shell-fixture-descendant:{address}"))
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    let mut connection = std::net::TcpStream::connect(address).unwrap();
    connection.write_all(b"P").unwrap();
    let _ = connection.read(&mut [0]);

    // Deliberately outlive the parent, retaining inherited output handles.
    if child.try_wait().unwrap().is_some() {
        std::process::exit(8);
    }

    std::process::exit(0);
}

fn run_flood() -> ! {
    let stderr = thread::spawn(|| {
        let block = vec![b'e'; 4096];
        for _ in 0..256 {
            std::io::stderr().write_all(&block).unwrap();
        }
    });

    let block = vec![b'o'; 4096];
    for _ in 0..256 {
        std::io::stdout().write_all(&block).unwrap();
    }
    stderr.join().unwrap();

    std::process::exit(0);
}
