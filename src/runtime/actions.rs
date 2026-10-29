use std::collections::VecDeque;

use crate::app::{Action, AppState, Effect, update};
use crate::config::ConfigRuntime;
use crate::theme::Theme;

use super::terminal_writer::TerminalWriter;
use super::{RuntimeResources, effects};

pub(super) fn process_actions(
    actions: &mut VecDeque<Action>,
    state: &mut AppState,
    config: &ConfigRuntime,
    mut resources: RuntimeResources<'_>,
    writer: &mut TerminalWriter,
    theme: &mut Theme,
    ui_dirty: &mut bool,
) -> Result<(), String> {
    while let Some(action) = actions.pop_front() {
        let changes_theme = matches!(
            &action,
            Action::UseTheme(_)
                | Action::SetDynamicFileColorPolicy(_)
                | Action::UsePalette(_)
                | Action::UseSyntax(_)
                | Action::UseDisplayStyle(_)
                | Action::ApplyDisplayStyle(_)
        );
        let changes_display = matches!(
            &action,
            Action::SetStylePreset(_)
                | Action::SetPanelStyle(_)
                | Action::SetFooterStyle(_)
                | Action::SetSeparatorStyle(_)
                | Action::SetBorderStyle(_)
                | Action::SetIconMode(_)
                | Action::SetNameColors(_)
        );
        let changes_help = matches!(
            &action,
            Action::HelpLoaded(_) | Action::ScrollHelp(_) | Action::CloseHelp
        );

        let viewer_was_ready = state.viewer.status == crate::preview::ViewerStatus::Ready;
        let effects = update(state, action);
        let viewer_remains_ready = state.viewer.status == crate::preview::ViewerStatus::Ready;
        *ui_dirty |= changes_theme
            || changes_display
            || changes_help
            || !viewer_was_ready
            || !viewer_remains_ready;

        for effect in effects {
            if matches!(
                effect,
                Effect::PreparePreviewMedia { .. }
                    | Effect::PrepareViewerMedia { .. }
                    | Effect::StartPreviewPlayback { .. }
                    | Effect::PresentDocumentRaster { .. }
            ) && *ui_dirty
            {
                *theme = session_theme(config, state);
                state.syntax_theme = config.syntax_theme()?;
                state.syntax_enabled = config.syntax_enabled();
                writer.draw(state, theme)?;
                *ui_dirty = false;
            }

            let next = effects::execute(effect, config, &mut resources, writer, theme);
            actions.extend(next);
        }
    }

    *theme = session_theme(config, state);
    state.syntax_theme = config.syntax_theme()?;
    state.syntax_enabled = config.syntax_enabled();

    Ok(())
}

fn session_theme(config: &ConfigRuntime, state: &AppState) -> Theme {
    let mut theme = config
        .theme()
        .unwrap_or_else(|_| Theme::emergency_monochrome());

    if let Some(style) = state.separator_style_override {
        theme.separator_style = style;
    }
    if let Some(style) = state.border_style_override {
        theme.set_border_style(style);
    }

    theme
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StandardPaths;
    use crate::terminal::TerminalSize;

    #[test]
    fn border_and_separator_overrides_survive_theme_switches_and_appearance_updates() {
        let paths = StandardPaths {
            config: "missing/config".into(),
            data: "missing/data".into(),
            state: "missing/state".into(),
            cache: "missing/cache".into(),
            runtime: "missing/runtime".into(),
        };
        let config = ConfigRuntime::load(paths, true).unwrap();
        let mut state = AppState::new(
            ".".into(),
            "test".into(),
            TerminalSize {
                columns: 80,
                rows: 24,
            },
            config.workspace_options(),
        );

        assert_eq!(
            session_theme(&config, &state).separator_style,
            config.theme().unwrap().separator_style
        );

        state.separator_style_override = Some(crate::theme::SeparatorStyle::Double);
        state.border_style_override = Some(crate::theme::BorderStyle::Simple);
        assert_eq!(
            session_theme(&config, &state).separator_style,
            crate::theme::SeparatorStyle::Double
        );
        assert!(session_theme(&config, &state).simple_border);

        config.use_theme("catppuccin_mocha").unwrap();
        assert_eq!(
            session_theme(&config, &state).separator_style,
            crate::theme::SeparatorStyle::Double
        );
        assert!(session_theme(&config, &state).simple_border);

        config.update_terminal_appearance(crate::terminal_appearance::TerminalAppearance {
            scheme: crate::terminal_appearance::Scheme::Light,
            background: None,
            palette: None,
        });
        config.use_theme("terminal").unwrap();
        assert_eq!(
            session_theme(&config, &state).separator_style,
            crate::theme::SeparatorStyle::Double
        );
        assert!(session_theme(&config, &state).simple_border);
    }
}
