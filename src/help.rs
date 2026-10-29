use crate::input::{InputContext, Keymaps};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpEntry {
    pub context: InputContext,
    pub keys: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpState {
    pub entries: Vec<HelpEntry>,
    pub scroll: usize,
}

impl HelpState {
    pub fn from_keymaps(keymaps: &Keymaps) -> Self {
        let entries = InputContext::ALL
            .into_iter()
            .flat_map(|context| {
                keymaps
                    .bindings(context)
                    .iter()
                    .map(move |binding| HelpEntry {
                        context,
                        keys: binding.display_keys(),
                        description: binding
                            .description
                            .clone()
                            .unwrap_or_else(|| "(no description)".to_owned()),
                    })
            })
            .collect();

        Self { entries, scroll: 0 }
    }

    pub fn scroll_by(&mut self, delta: isize, visible_rows: usize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(delta)
            .min(self.entries.len().saturating_sub(visible_rows));
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        self.scroll = self
            .scroll
            .min(self.entries.len().saturating_sub(visible_rows));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_includes_custom_bindings_and_clamps_scroll() {
        let mut keymaps = Keymaps::default();
        keymaps
            .set_with_description(
                InputContext::Normal,
                "y",
                "file.copy_to_other",
                Some("Copy selection short alias".into()),
                " ",
            )
            .unwrap();
        keymaps
            .set(InputContext::Viewer, "<Esc>", "viewer.close", " ")
            .unwrap();

        let mut help = HelpState::from_keymaps(&keymaps);
        assert_eq!(help.entries[0].keys, "y");
        assert_eq!(help.entries[0].description, "Copy selection short alias");
        assert_eq!(help.entries[1].context, InputContext::Viewer);
        assert_eq!(help.entries[1].description, "(no description)");

        help.scroll_by(isize::MAX, 1);
        assert_eq!(help.scroll, 1);
        help.scroll_by(isize::MIN, 1);
        assert_eq!(help.scroll, 0);
    }
}
