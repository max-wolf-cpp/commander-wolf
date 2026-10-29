use crossterm::event::KeyCode;

use super::keys::display_chord;
use super::{InputContext, KeyChord, Keymaps};
use crate::commands::CommandInvocation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolveOutcome {
    Pending(String),
    Command(CommandInvocation),
    Unmapped {
        chord: KeyChord,
        consumed_prefix: bool,
    },
}

#[derive(Clone, Debug, Default)]
pub struct KeyResolver {
    pending: Vec<KeyChord>,
    count: Option<usize>,
}

impl KeyResolver {
    pub fn reset(&mut self) {
        self.pending.clear();
        self.count = None;
    }

    pub fn resolve(
        &mut self,
        maps: &Keymaps,
        context: InputContext,
        chord: KeyChord,
    ) -> ResolveOutcome {
        if self.consume_count(context, &chord) {
            return ResolveOutcome::Pending(self.display());
        }

        let consumed_prefix = !self.pending.is_empty() || self.count.is_some();
        self.pending.push(chord.clone());
        let matches = maps.matches(context, &self.pending).collect::<Vec<_>>();

        if let Some(binding) = matches
            .iter()
            .find(|binding| binding.sequence.len() == self.pending.len())
        {
            let id = binding.command.clone();
            let count = self.count.map(|value| value.max(1));
            self.reset();

            return ResolveOutcome::Command(CommandInvocation::new(id, Vec::new(), count));
        }

        if !matches.is_empty() {
            return ResolveOutcome::Pending(self.display());
        }

        self.reset();

        ResolveOutcome::Unmapped {
            chord,
            consumed_prefix,
        }
    }

    pub fn display(&self) -> String {
        let mut text = self
            .count
            .map(|count| count.to_string())
            .unwrap_or_default();
        for chord in &self.pending {
            text.push_str(&display_chord(chord));
        }

        text
    }

    fn consume_count(&mut self, context: InputContext, chord: &KeyChord) -> bool {
        if !matches!(
            context,
            InputContext::Normal | InputContext::Visual | InputContext::FinderNormal
        ) || !self.pending.is_empty()
            || !chord.modifiers.is_empty()
        {
            return false;
        }

        let KeyCode::Char(character @ '0'..='9') = chord.code else {
            return false;
        };

        if self.count.is_none() && character == '0' {
            return false;
        }

        let digit = character.to_digit(10).unwrap_or_default() as usize;
        self.count = Some(
            self.count
                .unwrap_or_default()
                .saturating_mul(10)
                .saturating_add(digit),
        );

        true
    }
}
