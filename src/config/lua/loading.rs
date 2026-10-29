use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;
use std::rc::Rc;

use mlua::{Lua, Table};

use super::api::install_api;
use super::appearance;
use super::{ConfigDraft, ConfigRuntime, StandardPaths, lua_error};
use crate::terminal_appearance::TerminalAppearance;

impl ConfigRuntime {
    pub fn load(paths: StandardPaths, clean: bool) -> Result<Self, String> {
        let defaults = Self::build(&paths, None)
            .map_err(|error| format!("shipped Lua runtime failed: {error}"))?;
        if clean {
            return Ok(defaults);
        }

        let user_path = paths.config.join("init.lua");
        if !user_path.is_file() {
            return Ok(defaults);
        }

        match Self::build(&paths, Some(&user_path)) {
            Ok(runtime) => Ok(runtime),
            Err(error) => {
                let mut fallback = defaults;
                fallback.load_warning = Some(format!(
                    "{}: {error}\nUsing shipped defaults.",
                    user_path.display()
                ));

                Ok(fallback)
            }
        }
    }

    pub(super) fn build(paths: &StandardPaths, user_path: Option<&Path>) -> Result<Self, String> {
        let lua = Lua::new();
        let draft = Rc::new(RefCell::new(ConfigDraft::default()));
        let appearance = Rc::new(Cell::new(TerminalAppearance::default()));
        install_api(&lua, Rc::clone(&draft), Rc::clone(&appearance), paths)?;

        if user_path.is_some() {
            add_user_module_path(&lua, paths)?;
        }

        load_shipped(&lua)?;
        draft
            .borrow_mut()
            .internal_nearest_palettes
            .insert("far".into());

        if let Some(path) = user_path {
            load_user(&lua, path)?;
        }

        validate(&draft.borrow())?;

        Ok(Self {
            lua,
            draft,
            appearance,
            load_warning: None,
        })
    }
}

fn add_user_module_path(lua: &Lua, paths: &StandardPaths) -> Result<(), String> {
    let package: Table = lua.globals().get("package").map_err(lua_error)?;
    let previous: String = package.get("path").map_err(lua_error)?;
    let module = paths.config.join("?.lua");
    let nested = paths.config.join("?").join("init.lua");
    package
        .set(
            "path",
            format!("{};{};{previous}", module.display(), nested.display()),
        )
        .map_err(lua_error)?;

    Ok(())
}

fn load_user(lua: &Lua, path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read user configuration: {error}"))?;
    lua.load(&source)
        .set_name(format!("@{}", path.display()))
        .exec()
        .map_err(|error| error.to_string())
}

fn validate(draft: &ConfigDraft) -> Result<(), String> {
    appearance::validate_presets(draft)?;

    for command in draft.keymaps.command_ids() {
        if !draft.commands.contains(&command.0) {
            return Err(format!(
                "keymap references unknown command id: {}",
                command.0
            ));
        }
    }

    Ok(())
}

fn load_shipped(lua: &Lua) -> Result<(), String> {
    for (name, source) in [
        (
            "@runtime/lua/commander_wolf/themes/catppuccin_latte.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/catppuccin_latte.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/catppuccin_frappe.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/catppuccin_frappe.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/catppuccin_macchiato.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/catppuccin_macchiato.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/catppuccin_mocha.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/catppuccin_mocha.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/dracula.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/dracula.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/far.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/far.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/gruvbox_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/gruvbox_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/gruvbox_light.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/gruvbox_light.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/nord.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/nord.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/one_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/one_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/solarized_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/solarized_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/solarized_light.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/solarized_light.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/themes/tokyonight_night.lua",
            include_str!("../../../runtime/lua/commander_wolf/themes/tokyonight_night.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/catppuccin_latte.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/catppuccin_latte.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/catppuccin_frappe.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/catppuccin_frappe.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/catppuccin_macchiato.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/catppuccin_macchiato.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/catppuccin_mocha.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/catppuccin_mocha.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/dracula.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/dracula.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/far.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/far.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/gruvbox_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/gruvbox_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/gruvbox_light.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/gruvbox_light.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/nord.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/nord.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/one_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/one_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/solarized_dark.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/solarized_dark.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/solarized_light.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/solarized_light.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/syntax/tokyonight_night.lua",
            include_str!("../../../runtime/lua/commander_wolf/syntax/tokyonight_night.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/styles/nvim.lua",
            include_str!("../../../runtime/lua/commander_wolf/styles/nvim.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/styles/modern.lua",
            include_str!("../../../runtime/lua/commander_wolf/styles/modern.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/styles/classic.lua",
            include_str!("../../../runtime/lua/commander_wolf/styles/classic.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/defaults.lua",
            include_str!("../../../runtime/lua/commander_wolf/defaults.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/commands.lua",
            include_str!("../../../runtime/lua/commander_wolf/commands.lua"),
        ),
        (
            "@runtime/lua/commander_wolf/keymaps.lua",
            include_str!("../../../runtime/lua/commander_wolf/keymaps.lua"),
        ),
    ] {
        lua.load(source)
            .set_name(name)
            .exec()
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}
