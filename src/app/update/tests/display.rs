use super::*;

#[test]
fn theme_switch_requests_runtime_selection() {
    let mut state = state();
    assert_eq!(
        update(&mut state, Action::UseTheme("terminal".into())),
        vec![Effect::UseTheme("terminal".into())]
    );
}

#[test]
fn session_display_commands_update_only_the_live_state() {
    let mut state = state();
    assert_eq!(state.separator_style_override, None);
    assert_eq!(state.border_style_override, None);

    update(
        &mut state,
        Action::SetSeparatorStyle(crate::theme::SeparatorStyle::Thick),
    );
    update(&mut state, Action::SetIconMode(IconMode::None));
    update(
        &mut state,
        Action::SetBorderStyle(crate::theme::BorderStyle::Unicode(
            ratatui::widgets::BorderType::Double,
        )),
    );
    update(&mut state, Action::SetNameColors(true));

    assert_eq!(
        state.separator_style_override,
        Some(crate::theme::SeparatorStyle::Thick)
    );
    assert_eq!(state.workspace.icon_mode, IconMode::None);
    assert_eq!(
        state.border_style_override,
        Some(crate::theme::BorderStyle::Unicode(
            ratatui::widgets::BorderType::Double,
        ))
    );
    assert!(state.workspace.color_file_names);

    update(&mut state, Action::SetNameColors(false));
    assert!(!state.workspace.color_file_names);
}

#[test]
fn runtime_ui_style_switch_recalculates_panel_rows() {
    let mut state = state();
    let effects = update(
        &mut state,
        Action::SetStylePreset(crate::status_line::UiPreset::Borderless),
    );
    assert!(effects.is_empty());
    assert_eq!(
        state.status_line_options.style,
        crate::status_line::UiStyle::Borderless
    );
    assert_eq!(
        state.workspace.panel_style,
        crate::workspace::PanelStyle::Borderless
    );
    assert_eq!(state.workspace.active().viewport.visible_rows, 27);
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| { notification.message == "Style 'borderless' active" })
    );

    update(
        &mut state,
        Action::SetStylePreset(crate::status_line::UiPreset::Compact),
    );
    assert_eq!(
        state.status_line_options.style,
        crate::status_line::UiStyle::Compact
    );
    assert_eq!(
        state.workspace.panel_style,
        crate::workspace::PanelStyle::Compact
    );
    assert_eq!(state.workspace.active().viewport.visible_rows, 26);
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| { notification.message == "Style 'compact' active" })
    );

    update(
        &mut state,
        Action::SetStylePreset(crate::status_line::UiPreset::Framed),
    );
    assert_eq!(
        state.status_line_options.style,
        crate::status_line::UiStyle::Framed
    );
    assert_eq!(state.workspace.active().viewport.visible_rows, 23);
    assert_eq!(
        state.workspace.panel_style,
        crate::workspace::PanelStyle::Framed
    );

    update(
        &mut state,
        Action::SetFooterStyle(crate::status_line::UiStyle::Borderless),
    );
    assert_eq!(
        state.status_line_options.style,
        crate::status_line::UiStyle::Borderless
    );
    assert_eq!(
        state.workspace.panel_style,
        crate::workspace::PanelStyle::Framed
    );
    assert_eq!(state.workspace.active().viewport.visible_rows, 26);
    assert!(state.notification.as_ref().is_some_and(|notification| {
        notification.message == "Footer style 'borderless' active"
    }));

    update(
        &mut state,
        Action::SetPanelStyle(crate::workspace::PanelStyle::Compact),
    );
    assert_eq!(
        state.workspace.panel_style,
        crate::workspace::PanelStyle::Compact
    );
    assert_eq!(state.workspace.active().viewport.visible_rows, 27);
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| { notification.message == "Panel style 'compact' active" })
    );
}
