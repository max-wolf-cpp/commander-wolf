use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::commands::{CommandDispatch, CommandInvocation};
use crate::dialogs::DialogDisplay;
use crate::icons::IconMode;
use crate::input::{InputContext, KeyChord, KeyResolver, ResolveOutcome};
use crate::status_line::{ModePosition, UiPreset, UiStyle};
use crate::syntax::SyntaxTheme;
use crate::theme::{BorderStyle, Rgb, SeparatorStyle, Theme};
use crate::workspace::{LineNumberMode, PanelStyle};
use crossterm::event::{KeyCode, KeyModifiers};

mod appearance;
mod applications;
mod commands;
mod components;
mod keymaps;
mod loading;
mod options;
mod presets;

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "commander-wolf-config-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();

        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn paths(root: &Path) -> StandardPaths {
    StandardPaths {
        config: root.join("config/commander-wolf"),
        data: root.join("data/commander-wolf"),
        state: root.join("state/commander-wolf"),
        cache: root.join("cache/commander-wolf"),
        runtime: root.join("runtime"),
    }
}
