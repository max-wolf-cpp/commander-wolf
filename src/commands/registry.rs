use std::collections::HashMap;

use super::builtin_catalog::builtin_commands;
use super::completion::{CompletionItem, command_prefix};
use super::dispatch::builtin_action;
use super::display::fixed_argument_values;
use super::{BuiltinCommand, CommandId, CommandInvocation};
use crate::app::Action;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Handler {
    Builtin(BuiltinCommand),
    Lua(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandDispatch {
    Actions(Vec<Action>),
    Lua {
        callback: u64,
        invocation: CommandInvocation,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    handlers: HashMap<CommandId, Handler>,
    aliases: HashMap<String, CommandId>,
    descriptions: HashMap<CommandId, String>,
}

impl CommandRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        for (id, command, description) in builtin_commands() {
            registry.define_builtin(id, command, description);
        }

        registry
    }

    pub fn create_lua(
        &mut self,
        id: impl Into<String>,
        callback: u64,
        description: impl Into<String>,
    ) {
        let id = CommandId(id.into());
        self.handlers.insert(id.clone(), Handler::Lua(callback));
        self.descriptions.insert(id, description.into());
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.aliases.remove(id);
        let id = CommandId(id.to_owned());
        if matches!(self.handlers.get(&id), Some(Handler::Builtin(_))) {
            return Err(format!("cannot delete built-in command id: {}", id.0));
        }

        self.handlers.remove(&id);
        self.descriptions.remove(&id);
        self.aliases.retain(|_, target| target != &id);

        Ok(())
    }

    pub fn alias(&mut self, name: impl Into<String>, id: impl Into<String>) -> Result<(), String> {
        let id = CommandId(id.into());
        if !self.handlers.contains_key(&id) {
            return Err(format!("unknown command id: {}", id.0));
        }

        self.aliases.insert(name.into(), id);

        Ok(())
    }

    pub fn delete_alias(&mut self, name: &str) {
        self.aliases.remove(name);
    }

    pub fn resolve_alias(&self, name: &str) -> Option<&CommandId> {
        self.aliases.get(name)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.handlers.contains_key(&CommandId(id.to_owned()))
    }

    pub fn description(&self, id: &CommandId) -> Option<&str> {
        self.descriptions.get(id).map(String::as_str)
    }

    pub fn completions(&self, line: &str) -> Vec<CompletionItem> {
        let mut items = if let Some(prefix) = command_prefix(line) {
            self.complete_names(prefix)
        } else {
            self.complete_arguments(line)
        };
        items.sort_by(|left, right| left.name.cmp(&right.name));

        items
    }

    pub fn dispatch(&self, invocation: CommandInvocation) -> Result<CommandDispatch, String> {
        match self.handlers.get(&invocation.id) {
            Some(Handler::Builtin(command)) => Ok(CommandDispatch::Actions(vec![builtin_action(
                *command,
                &invocation,
            )?])),
            Some(Handler::Lua(callback)) => Ok(CommandDispatch::Lua {
                callback: *callback,
                invocation,
            }),
            None => Err(format!("unknown command id: {}", invocation.id.0)),
        }
    }

    fn define_builtin(&mut self, id: &str, command: BuiltinCommand, description: &str) {
        let id = CommandId::from(id);
        self.handlers.insert(id.clone(), Handler::Builtin(command));
        self.descriptions.insert(id, description.to_owned());
    }

    fn complete_names(&self, prefix: &str) -> Vec<CompletionItem> {
        self.aliases
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(name, id)| CompletionItem {
                name: name.clone(),
                description: self.description(id).unwrap_or_default().to_owned(),
            })
            .collect()
    }

    fn complete_arguments(&self, line: &str) -> Vec<CompletionItem> {
        let input = line.strip_prefix(':').unwrap_or(line).trim_start();
        if input.contains(['\'', '"', '\\']) {
            return Vec::new();
        }

        let words = input.split_whitespace().collect::<Vec<_>>();
        let Some(alias) = words.first() else {
            return Vec::new();
        };

        let Some(Handler::Builtin(command)) = self
            .aliases
            .get(*alias)
            .and_then(|id| self.handlers.get(id))
        else {
            return Vec::new();
        };

        let trailing_space = line.chars().last().is_some_and(char::is_whitespace);
        let (arguments, prefix) = if trailing_space {
            (&words[1..], "")
        } else {
            (&words[1..words.len() - 1], words[words.len() - 1])
        };

        fixed_argument_values(*command, arguments)
            .iter()
            .filter(|value| value.starts_with(prefix))
            .map(|value| CompletionItem {
                name: (*value).to_owned(),
                description: String::new(),
            })
            .collect()
    }
}

#[cfg(test)]
mod completion_tests;
#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod display_tests;
#[cfg(test)]
mod file_tests;
#[cfg(test)]
mod navigation_tests;
