use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mlua::{Lua, Table, Value};

use super::applications::file_association_api;
use super::commands::{application_api, command_api};
use super::components::{ThemePart, component_api};
use super::keymaps::keymap_api;
use super::options::option_api;
use super::{CALLBACKS_KEY, ConfigDraft, StandardPaths, lua_error};
use crate::terminal_appearance::TerminalAppearance;

pub(super) fn install_api(
    lua: &Lua,
    draft: Rc<RefCell<ConfigDraft>>,
    appearance: Rc<Cell<TerminalAppearance>>,
    paths: &StandardPaths,
) -> Result<(), String> {
    let callbacks = lua.create_table().map_err(lua_error)?;
    lua.set_named_registry_value(CALLBACKS_KEY, callbacks)
        .map_err(lua_error)?;

    let wolf = lua.create_table().map_err(lua_error)?;
    wolf.set("g", globals_api(lua, Rc::clone(&draft))?)
        .map_err(lua_error)?;
    wolf.set("fn", path_api(lua, paths)?).map_err(lua_error)?;
    wolf.set("keymap", keymap_api(lua, Rc::clone(&draft))?)
        .map_err(lua_error)?;
    wolf.set("command", command_api(lua, Rc::clone(&draft))?)
        .map_err(lua_error)?;

    for (name, part) in [
        ("theme", ThemePart::Theme),
        ("syntax", ThemePart::Syntax),
        ("style", ThemePart::Style),
    ] {
        let component = component_api(
            lua,
            Rc::clone(&draft),
            Rc::clone(&appearance),
            part,
            String::new(),
        )
        .map_err(lua_error)?;
        wolf.set(name, component).map_err(lua_error)?;
    }

    wolf.set("option", option_api(lua, Rc::clone(&draft))?)
        .map_err(lua_error)?;
    wolf.set(
        "file_association",
        file_association_api(lua, Rc::clone(&draft))?,
    )
    .map_err(lua_error)?;
    wolf.set("api", application_api(lua, Rc::clone(&draft))?)
        .map_err(lua_error)?;

    lua.globals().set("wolf", wolf.clone()).map_err(lua_error)?;
    let package: Table = lua.globals().get("package").map_err(lua_error)?;
    let loaded: Table = package.get("loaded").map_err(lua_error)?;
    loaded.set("commander_wolf", wolf).map_err(lua_error)?;

    Ok(())
}

fn globals_api(lua: &Lua, draft: Rc<RefCell<ConfigDraft>>) -> Result<Table, String> {
    let globals_proxy = lua.create_table().map_err(lua_error)?;
    let globals_meta = lua.create_table().map_err(lua_error)?;
    let read_draft = Rc::clone(&draft);
    globals_meta
        .set(
            "__index",
            lua.create_function(move |lua, (_table, key): (Table, String)| {
                if key == "mapleader" {
                    Ok(Value::String(
                        lua.create_string(&read_draft.borrow().leader)?,
                    ))
                } else {
                    Ok(Value::Nil)
                }
            })
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    let write_draft = Rc::clone(&draft);
    globals_meta
        .set(
            "__newindex",
            lua.create_function(move |_, (_table, key, value): (Table, String, Value)| {
                if key != "mapleader" {
                    return Err(mlua::Error::runtime(format!(
                        "unknown wolf.g option: {key}"
                    )));
                }

                let Value::String(value) = value else {
                    return Err(mlua::Error::runtime("wolf.g.mapleader must be a string"));
                };

                write_draft.borrow_mut().leader = value.to_str()?.to_owned();
                Ok(())
            })
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;
    globals_proxy
        .set_metatable(Some(globals_meta))
        .map_err(lua_error)?;

    Ok(globals_proxy)
}

fn path_api(lua: &Lua, paths: &StandardPaths) -> Result<Table, String> {
    let path_values = paths.clone();
    let functions = lua.create_table().map_err(lua_error)?;
    functions
        .set(
            "stdpath",
            lua.create_function(move |_, name: String| {
                path_values
                    .get(&name)
                    .map(|path| path.to_string_lossy().into_owned())
                    .ok_or_else(|| mlua::Error::runtime(format!("unknown standard path: {name}")))
            })
            .map_err(lua_error)?,
        )
        .map_err(lua_error)?;

    Ok(functions)
}
