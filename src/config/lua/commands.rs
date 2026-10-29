use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Function, Lua, Table, Value};

use super::{CALLBACKS_KEY, ConfigDraft, ConfigRuntime, LuaRequest, lua_error};
use crate::commands::completion::CompletionItem;
use crate::commands::{CommandDispatch, CommandId, CommandInvocation};

impl ConfigRuntime {
    pub fn dispatch(
        &self,
        id: &CommandId,
        args: Vec<String>,
        count: Option<usize>,
    ) -> Result<CommandDispatch, String> {
        self.draft
            .borrow()
            .commands
            .dispatch(CommandInvocation::new(id.clone(), args, count))
    }

    pub fn command_line(&self, line: &str) -> Result<CommandDispatch, String> {
        let parsed = crate::commands::parse_command_line(line)?;
        let draft = self.draft.borrow();
        let id = draft
            .commands
            .resolve_alias(&parsed.name)
            .ok_or_else(|| format!("Not a Commander Wolf command: {}", parsed.name))?;

        draft
            .commands
            .dispatch(CommandInvocation::new(id.clone(), parsed.args, None))
    }

    pub fn command_completions(&self, line: &str) -> Vec<CompletionItem> {
        let draft = self.draft.borrow();
        preset_completions(&draft, line).unwrap_or_else(|| draft.commands.completions(line))
    }

    pub fn call_lua(
        &self,
        callback: u64,
        args: Vec<String>,
        count: Option<usize>,
    ) -> Result<Vec<LuaRequest>, String> {
        let callbacks: Table = self
            .lua
            .named_registry_value(CALLBACKS_KEY)
            .map_err(|error| error.to_string())?;
        let function: Function = callbacks.get(callback).map_err(|error| error.to_string())?;

        let context = self.lua.create_table().map_err(|error| error.to_string())?;
        context
            .set("args", args)
            .map_err(|error| error.to_string())?;
        context
            .set("count", count)
            .map_err(|error| error.to_string())?;

        function
            .call::<()>(context)
            .map_err(|error| error.to_string())?;

        Ok(std::mem::take(&mut self.draft.borrow_mut().requests))
    }

    pub fn take_requests(&self) -> Vec<LuaRequest> {
        std::mem::take(&mut self.draft.borrow_mut().requests)
    }
}

pub(super) fn command_api(lua: &Lua, draft: Rc<RefCell<ConfigDraft>>) -> Result<Table, String> {
    let api = lua.create_table().map_err(lua_error)?;
    let alias_draft = Rc::clone(&draft);
    api.set(
        "alias",
        lua.create_function(move |_, (name, id): (String, String)| {
            alias_draft
                .borrow_mut()
                .commands
                .alias(name, id)
                .map_err(mlua::Error::runtime)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let create_draft = Rc::clone(&draft);
    api.set(
        "create",
        lua.create_function(
            move |lua, (name, callback, options): (String, Function, Option<Table>)| {
                let mut draft = create_draft.borrow_mut();
                let id = draft.next_callback;
                draft.next_callback += 1;
                let callbacks: Table = lua.named_registry_value(CALLBACKS_KEY)?;
                callbacks.set(id, callback)?;

                let description = options
                    .and_then(|table| table.get::<String>("description").ok())
                    .unwrap_or_default();
                draft.commands.create_lua(&name, id, description);
                draft
                    .commands
                    .alias(name.clone(), name)
                    .map_err(mlua::Error::runtime)
            },
        )
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let delete_draft = Rc::clone(&draft);
    api.set(
        "del",
        lua.create_function(move |_, name: String| {
            delete_draft
                .borrow_mut()
                .commands
                .delete(&name)
                .map_err(mlua::Error::runtime)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let execute_draft = Rc::clone(&draft);
    api.set(
        "execute",
        lua.create_function(move |_, values: mlua::Variadic<Value>| {
            let (name, args) = command_arguments(values)?;
            let mut draft = execute_draft.borrow_mut();
            let command = draft
                .commands
                .resolve_alias(&name)
                .cloned()
                .unwrap_or(CommandId(name));
            draft.requests.push(LuaRequest::Execute {
                command,
                args,
                count: None,
            });
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    Ok(api)
}

pub(super) fn application_api(lua: &Lua, draft: Rc<RefCell<ConfigDraft>>) -> Result<Table, String> {
    let api = lua.create_table().map_err(lua_error)?;
    let notify_draft = Rc::clone(&draft);
    api.set(
        "notify",
        lua.create_function(move |_, message: String| {
            notify_draft
                .borrow_mut()
                .requests
                .push(LuaRequest::Notify(message));
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    api.set(
        "command",
        lua.create_function(move |_, values: mlua::Variadic<Value>| {
            let (command, args) = command_arguments(values)?;
            draft.borrow_mut().requests.push(LuaRequest::Execute {
                command: CommandId(command),
                args,
                count: None,
            });
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    Ok(api)
}

fn preset_completions(draft: &ConfigDraft, line: &str) -> Option<Vec<CompletionItem>> {
    let input = line.strip_prefix(':').unwrap_or(line).trim_start();
    if input.contains(['\'', '"', '\\']) {
        return None;
    }

    let words = input.split_whitespace().collect::<Vec<_>>();
    let trailing_space = line.chars().last().is_some_and(char::is_whitespace);
    let (alias, prefix) = match (words.as_slice(), trailing_space) {
        ([alias], true) => (*alias, ""),
        ([alias, prefix], false) => (*alias, *prefix),
        _ => return None,
    };

    let command = draft.commands.resolve_alias(alias)?;
    let names = preset_names(draft, command);
    if names.is_empty() {
        return None;
    }

    let mut names = names
        .into_iter()
        .filter(|name| !name.starts_with('@') && name.starts_with(prefix))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();

    Some(
        names
            .into_iter()
            .map(|name| CompletionItem {
                name,
                description: String::new(),
            })
            .collect(),
    )
}

fn preset_names(draft: &ConfigDraft, command: &CommandId) -> Vec<String> {
    match command.0.as_str() {
        "palette.use" => draft
            .themes
            .keys()
            .cloned()
            .chain(["terminal".into()])
            .collect(),
        "syntax.use" => draft
            .syntax_themes
            .keys()
            .cloned()
            .chain(["terminal".into(), "off".into()])
            .collect(),
        "display_style.use" => draft.styles.keys().cloned().collect(),
        "theme.use" => draft
            .theme_specs
            .keys()
            .cloned()
            .chain(["terminal".into()])
            .collect(),
        _ => Vec::new(),
    }
}

fn command_arguments(values: mlua::Variadic<Value>) -> mlua::Result<(String, Vec<String>)> {
    let mut values = values.into_iter();
    let name = match values.next() {
        Some(Value::String(value)) => value.to_str()?.to_owned(),
        _ => return Err(mlua::Error::runtime("command name must be a string")),
    };

    let mut args = Vec::new();
    for value in values {
        match value {
            Value::Table(table) => {
                for value in table.sequence_values::<Value>() {
                    args.push(command_argument(value?)?);
                }
            }
            value => args.push(command_argument(value)?),
        }
    }

    Ok((name, args))
}

fn command_argument(value: Value) -> mlua::Result<String> {
    match value {
        Value::String(value) => value.to_str().map(|value| value.to_owned()),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Boolean(value) => Ok(value.to_string()),
        _ => Err(mlua::Error::runtime(
            "command arguments must be strings, numbers, booleans, or an array of them",
        )),
    }
}
