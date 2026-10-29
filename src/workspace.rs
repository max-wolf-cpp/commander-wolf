mod navigation;
mod options;
mod panel;
mod viewport;

use crate::icons::IconMode;

pub use navigation::{SelectionMotion, apply_selection_motion};
pub use options::{LineNumberMode, PanelStyle, WorkspaceOptions};
pub use panel::{DirectoryEntry, DirectorySnapshot, PanelState};
pub use viewport::ViewportState;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelId {
    Left,
    Right,
}

impl PanelId {
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceState {
    pub color_file_names: bool,
    pub show_hidden: bool,
    pub panels: [PanelState; 2],
    pub active_panel: PanelId,
    pub line_number_mode: LineNumberMode,
    pub scroll_offset: usize,
    pub icon_mode: IconMode,
    pub panel_style: PanelStyle,
    pub preview_visible: bool,
}

impl WorkspaceState {
    pub fn new(options: WorkspaceOptions) -> Self {
        Self {
            color_file_names: options.color_file_names,
            show_hidden: options.show_hidden,
            panels: std::array::from_fn(|_| PanelState {
                hide_hidden: !options.show_hidden,
                ..PanelState::default()
            }),
            active_panel: PanelId::Left,
            line_number_mode: options.line_numbers,
            scroll_offset: options.scroll_offset,
            icon_mode: options.icon_mode,
            panel_style: options.panel_style,
            preview_visible: false,
        }
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        for panel in &mut self.panels {
            panel.set_visible_rows(rows, self.scroll_offset);
        }
    }

    pub fn panel(&self, id: PanelId) -> &PanelState {
        &self.panels[id.index()]
    }

    pub fn panel_mut(&mut self, id: PanelId) -> &mut PanelState {
        &mut self.panels[id.index()]
    }

    pub fn active(&self) -> &PanelState {
        self.panel(self.active_panel)
    }

    pub fn active_mut(&mut self) -> &mut PanelState {
        self.panel_mut(self.active_panel)
    }

    pub fn preview_panel(&self) -> Option<PanelId> {
        self.preview_visible.then(|| self.active_panel.other())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_keep_independent_selection_and_viewports() {
        let mut workspace = WorkspaceState::new(WorkspaceOptions {
            color_file_names: false,
            show_hidden: true,
            line_numbers: LineNumberMode::Relative,
            scroll_offset: 10,
            icon_mode: IconMode::NerdFont,
            panel_style: PanelStyle::Framed,
        });
        workspace.panel_mut(PanelId::Left).selected = 4;
        workspace.panel_mut(PanelId::Left).viewport.scroll = 2;
        workspace.panel_mut(PanelId::Left).directory = "left".into();
        workspace.panel_mut(PanelId::Right).selected = 1;
        workspace.panel_mut(PanelId::Right).directory = "right".into();

        assert_eq!(workspace.panel(PanelId::Left).selected, 4);
        assert_eq!(workspace.panel(PanelId::Left).viewport.scroll, 2);
        assert_eq!(workspace.panel(PanelId::Right).selected, 1);
        assert_eq!(workspace.panel(PanelId::Right).viewport.scroll, 0);
        assert_eq!(
            workspace.panel(PanelId::Left).directory,
            std::path::Path::new("left")
        );
        assert_eq!(
            workspace.panel(PanelId::Right).directory,
            std::path::Path::new("right")
        );
    }
}
