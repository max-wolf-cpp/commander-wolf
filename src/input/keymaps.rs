use std::collections::HashMap;

use super::keys::{display_chord, parse_sequence};
use super::{InputContext, KeyChord};
use crate::commands::CommandId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeymapBinding {
    pub sequence: Vec<KeyChord>,
    pub command: CommandId,
    pub description: Option<String>,
}

impl KeymapBinding {
    pub fn display_keys(&self) -> String {
        self.sequence.iter().map(display_chord).collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Keymaps {
    bindings: HashMap<InputContext, Vec<KeymapBinding>>,
}

impl Keymaps {
    pub fn set(
        &mut self,
        context: InputContext,
        lhs: &str,
        command: impl Into<String>,
        leader: &str,
    ) -> Result<(), String> {
        self.set_with_description(context, lhs, command, None, leader)
    }

    pub fn set_with_description(
        &mut self,
        context: InputContext,
        lhs: &str,
        command: impl Into<String>,
        description: Option<String>,
        leader: &str,
    ) -> Result<(), String> {
        let sequence = parse_sequence(lhs, leader)?;
        let bindings = self.bindings.entry(context).or_default();
        bindings.retain(|binding| binding.sequence != sequence);
        bindings.push(KeymapBinding {
            sequence,
            command: CommandId(command.into()),
            description,
        });

        Ok(())
    }

    pub fn delete(&mut self, context: InputContext, lhs: &str, leader: &str) -> Result<(), String> {
        let sequence = parse_sequence(lhs, leader)?;
        if let Some(bindings) = self.bindings.get_mut(&context) {
            bindings.retain(|binding| binding.sequence != sequence);
        }

        Ok(())
    }

    pub fn clear(&mut self, context: InputContext) {
        self.bindings.remove(&context);
    }

    pub fn bindings(&self, context: InputContext) -> &[KeymapBinding] {
        self.bindings
            .get(&context)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn command_ids(&self) -> impl Iterator<Item = &CommandId> {
        self.bindings
            .values()
            .flatten()
            .map(|binding| &binding.command)
    }

    pub(super) fn matches<'a>(
        &'a self,
        context: InputContext,
        pending: &[KeyChord],
    ) -> impl Iterator<Item = &'a KeymapBinding> {
        self.bindings
            .get(&context)
            .into_iter()
            .flatten()
            .filter(move |binding| binding.sequence.starts_with(pending))
    }
}
