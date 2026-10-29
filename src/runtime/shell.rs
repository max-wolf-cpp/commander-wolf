use std::path::Path;
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crate::app::Action;
use crate::file_opening::Application;

use self::launch::spawn;
use self::output::{Decoder, OutputEvent, read_output};

mod launch;
mod output;

#[derive(Default)]
pub(super) struct ShellRuntime {
    running: Option<Running>,
}

struct Running {
    child: Child,
    receiver: Receiver<OutputEvent>,
    decoders: [Decoder; 2],
    closed: usize,
    status: Option<ExitStatus>,
    tree: super::process_tree::ProcessTree,
    readers: Vec<thread::JoinHandle<()>>,
    cancelled: bool,
}

impl ShellRuntime {
    pub fn start(
        &mut self,
        source: &str,
        directory: &Path,
        shell: Option<Application>,
    ) -> Vec<Action> {
        if self.running.is_some() {
            return vec![Action::ShellFinished {
                message: "A shell command is already running".into(),
                failed: true,
            }];
        }

        match spawn(source, directory, shell) {
            Ok((mut child, tree)) => {
                let (sender, receiver) = mpsc::sync_channel(64);
                let readers = vec![
                    read_output(child.stdout.take().unwrap(), sender.clone(), 0),
                    read_output(child.stderr.take().unwrap(), sender, 1),
                ];
                self.running = Some(Running {
                    child,
                    receiver,
                    decoders: Default::default(),
                    closed: 0,
                    status: None,
                    tree,
                    readers,
                    cancelled: false,
                });
                Vec::new()
            }
            Err(error) => vec![Action::ShellFinished {
                message: format!("Cannot start shell: {error}"),
                failed: true,
            }],
        }
    }

    pub fn poll(&mut self) -> Vec<Action> {
        let Some(running) = &mut self.running else {
            return Vec::new();
        };

        let mut actions = Vec::new();
        running.collect_output(&mut actions);

        if running.status.is_none() {
            match running.child.try_wait() {
                Ok(Some(status)) => {
                    running.status = Some(status);
                    if let Err(error) = running.tree.terminate() {
                        actions.push(Action::ShellOutput(format!(
                            "\nCannot stop descendants: {error}\n"
                        )));
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    actions.push(Action::ShellFinished {
                        message: format!("Cannot wait for shell: {error}"),
                        failed: true,
                    });
                    self.shutdown();
                    return actions;
                }
            }
        }

        if let Some(action) = running.completion() {
            actions.push(action);
            self.shutdown();
        }

        actions
    }

    pub fn cancel(&mut self) -> Vec<Action> {
        let Some(running) = &mut self.running else {
            return Vec::new();
        };

        if running.cancelled {
            return Vec::new();
        }

        running.cancelled = true;
        if let Err(error) = running.tree.terminate() {
            let fallback = running.child.kill();
            return vec![Action::ShellOutput(format!(
                "\nCannot stop process tree: {error}; shell termination: {fallback:?}\n"
            ))];
        }

        Vec::new()
    }

    fn shutdown(&mut self) {
        if let Some(mut running) = self.running.take() {
            let _ = running.tree.terminate();
            let _ = running.child.kill();
            let _ = running.child.wait();

            // Unblock readers waiting to send to the bounded channel before joining.
            drop(running.receiver);
            for reader in running.readers {
                let _ = reader.join();
            }
        }
    }
}

impl Drop for ShellRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Running {
    fn collect_output(&mut self, actions: &mut Vec<Action>) {
        // Bound work per event-loop turn as well as the channel's memory usage.
        for event in self.receiver.try_iter().take(64) {
            match event {
                OutputEvent::Data(stream, bytes) => {
                    let text = self.decoders[stream].push(&bytes, false);
                    if !text.is_empty() {
                        actions.push(Action::ShellOutput(text));
                    }
                }
                OutputEvent::Closed(stream) => {
                    self.closed += 1;
                    let text = self.decoders[stream].push(&[], true);
                    if !text.is_empty() {
                        actions.push(Action::ShellOutput(text));
                    }
                }
                OutputEvent::Error(error) => actions.push(Action::ShellOutput(format!(
                    "\nCannot read output: {error}\n"
                ))),
            }
        }
    }

    fn completion(&self) -> Option<Action> {
        let status = self.status?;
        if self.closed != 2 {
            return None;
        }

        Some(Action::ShellFinished {
            message: if self.cancelled {
                "Command interrupted".into()
            } else if let Some(code) = status.code() {
                format!("Exit code: {code}")
            } else {
                "Command terminated by a signal".into()
            },
            failed: self.cancelled || !status.success(),
        })
    }
}

#[cfg(test)]
mod tests;
