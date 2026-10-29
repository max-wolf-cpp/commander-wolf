use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Table, Value};

use super::{CALLBACKS_KEY, ConfigDraft, ConfigRuntime, lua_error};
use crate::input::{InputContext, Keymaps};

impl ConfigRuntime {
    pub fn leader(&self) -> String {
        self.draft.borrow().leader.clone()
    }

    pub fn keymaps(&self) -> std::cell::Ref<'_, Keymaps> {
        std::cell::Ref::map(self.draft.borrow(), |draft| &draft.keymaps)
    }
}

pub(super) fn keymap_api(lua: &Lua, draft: Rc<RefCell<ConfigDraft>>) -> Result<Table, String> {
    let api = lua.create_table().map_err(lua_error)?;
    let set_draft = Rc::clone(&draft);
    api.set(
        "set",
        lua.create_function(
            move |lua, (mode, lhs, rhs, options): (String, String, Value, Option<Table>)| {
                let context = InputContext::parse(&mode).map_err(mlua::Error::runtime)?;
                let description = match options
                    .as_ref()
                    .map(|options| options.get::<Value>("description"))
                    .transpose()?
                {
                    None | Some(Value::Nil) => None,
                    Some(Value::String(value)) => Some(value.to_str()?.to_owned()),
                    Some(_) => {
                        return Err(mlua::Error::runtime("keymap description must be a string"));
                    }
                };

                let mut draft = set_draft.borrow_mut();
                let command = match rhs {
                    Value::String(value) => value.to_str()?.to_owned(),
                    Value::Function(function) => {
                        let id = draft.next_callback;
                        draft.next_callback += 1;
                        let callbacks: Table = lua.named_registry_value(CALLBACKS_KEY)?;
                        callbacks.set(id, function)?;
                        let command = format!("lua.keymap.{id}");
                        draft
                            .commands
                            .create_lua(&command, id, "Lua keymap callback");
                        command
                    }
                    _ => {
                        return Err(mlua::Error::runtime(
                            "keymap rhs must be a command id or function",
                        ));
                    }
                };

                let leader = draft.leader.clone();
                draft
                    .keymaps
                    .set_with_description(context, &lhs, command, description, &leader)
                    .map_err(mlua::Error::runtime)
            },
        )
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let del_draft = Rc::clone(&draft);
    api.set(
        "del",
        lua.create_function(move |_, (mode, lhs): (String, String)| {
            let context = InputContext::parse(&mode).map_err(mlua::Error::runtime)?;
            let mut draft = del_draft.borrow_mut();
            let leader = draft.leader.clone();
            draft
                .keymaps
                .delete(context, &lhs, &leader)
                .map_err(mlua::Error::runtime)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    api.set(
        "clear",
        lua.create_function(move |_, mode: String| {
            let context = InputContext::parse(&mode).map_err(mlua::Error::runtime)?;
            draft.borrow_mut().keymaps.clear(context);
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    Ok(api)
}
