use super::overlays::{
    delete_float_action, directory_edit_float_action, file_conflict_float_action,
};
use super::*;
use crate::config::StandardPaths;
use crate::dialogs::DialogDisplay;

mod mappings;
mod overlays;

fn test_config() -> ConfigRuntime {
    ConfigRuntime::load(
        StandardPaths {
            config: "missing/config".into(),
            data: "missing/data".into(),
            state: "missing/state".into(),
            cache: "missing/cache".into(),
            runtime: "missing/runtime".into(),
        },
        true,
    )
    .unwrap()
}

fn test_state(config: &ConfigRuntime) -> AppState {
    AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        config.workspace_options(),
    )
}
