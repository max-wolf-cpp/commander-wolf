use std::collections::{HashMap, HashSet};

use super::{ConfigDraft, ConfigRuntime};
use crate::display_style::DisplayStyle;
use crate::syntax::SyntaxTheme;
use crate::terminal_appearance::TerminalAppearance;
use crate::theme::{Theme, derive_default_theme};

#[derive(Clone, Debug)]
pub(super) struct ComponentDefinition {
    pub(super) parent: Option<String>,
    pub(super) fields: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub(super) struct ThemeSpec {
    pub(super) syntax: Option<String>,
}

impl Default for ThemeSpec {
    fn default() -> Self {
        Self {
            syntax: Some("terminal".into()),
        }
    }
}

impl ConfigRuntime {
    pub fn update_terminal_appearance(&self, appearance: TerminalAppearance) -> bool {
        if self.appearance.get() == appearance {
            return false;
        }

        self.appearance.set(appearance);
        true
    }

    pub fn theme(&self) -> Result<Theme, String> {
        effective_theme(&self.draft.borrow(), self.appearance.get())
    }

    pub fn use_theme(&self, name: &str) -> Result<(), String> {
        select_theme(&mut self.draft.borrow_mut(), name, self.appearance.get())
    }

    pub fn use_palette(&self, name: &str) -> Result<(), String> {
        select_palette(&mut self.draft.borrow_mut(), name, self.appearance.get())
    }

    pub fn set_file_color_policy(
        &self,
        policy: crate::theme::DynamicFileColorPolicy,
    ) -> Result<(), String> {
        let mut draft = self.draft.borrow_mut();
        let active = draft.active_theme.as_ref().ok_or("no active theme")?;
        let name = draft.session_palette.as_deref().unwrap_or(active);
        if !palette_is_generated(&draft, name)? {
            return Err(":file-color-policy only applies to the generated terminal theme".into());
        }

        draft.session_file_color_policy = Some(policy);
        Ok(())
    }

    pub fn use_syntax(&self, name: &str) -> Result<(), String> {
        select_syntax(&mut self.draft.borrow_mut(), name)
    }

    pub fn syntax_enabled(&self) -> bool {
        self.draft
            .borrow()
            .session_syntax
            .as_ref()
            .is_none_or(Option::is_some)
    }

    pub fn syntax_theme(&self) -> Result<SyntaxTheme, String> {
        effective_syntax(&self.draft.borrow())
    }

    pub fn use_style(&self, name: &str) -> Result<(), String> {
        select_style(&mut self.draft.borrow_mut(), name)
    }

    pub fn display_style(&self) -> Result<DisplayStyle, String> {
        effective_style(&self.draft.borrow())
    }
}

pub(super) fn validate_presets(draft: &ConfigDraft) -> Result<(), String> {
    let name = draft
        .active_theme
        .as_deref()
        .ok_or("configuration did not select a theme")?;
    theme_spec(draft, name)?;

    for name in draft.theme_specs.keys() {
        resolve_palette(draft, name, TerminalAppearance::default())?;
        resolve_syntax(draft, &theme_syntax_name(draft, name)?)?;
    }
    for name in draft.themes.keys() {
        resolve_palette(draft, name, TerminalAppearance::default())?;
    }

    for name in draft.syntax_themes.keys() {
        resolve_syntax(draft, name)?;
    }

    for name in draft.styles.keys() {
        resolve_style(draft, name)?;
    }

    Ok(())
}

fn synthetic_theme_name(name: &str) -> bool {
    name == "terminal"
}

fn theme_spec(draft: &ConfigDraft, name: &str) -> Result<ThemeSpec, String> {
    if let Some(spec) = draft.theme_specs.get(name) {
        Ok(spec.clone())
    } else if synthetic_theme_name(name) {
        Ok(ThemeSpec::default())
    } else {
        Err(format!("unknown theme: {name}"))
    }
}

fn theme_syntax_name(draft: &ConfigDraft, name: &str) -> Result<String, String> {
    fn resolve(
        draft: &ConfigDraft,
        name: &str,
        seen: &mut HashSet<String>,
    ) -> Result<String, String> {
        if !seen.insert(name.into()) {
            return Err(format!("theme inheritance cycle at '{name}'"));
        }

        let spec = theme_spec(draft, name)?;
        let result = if let Some(syntax) = spec.syntax {
            Ok(syntax)
        } else if let Some(parent) = draft.palette_parents.get(name) {
            resolve(draft, parent, seen)
        } else {
            Err(format!(
                "theme '{name}' must define syntax or extend another theme"
            ))
        };
        seen.remove(name);

        result
    }

    resolve(draft, name, &mut HashSet::new())
}

pub(super) fn effective_palette(
    draft: &ConfigDraft,
    appearance: TerminalAppearance,
) -> Result<Theme, String> {
    let active = draft.active_theme.as_deref().ok_or("no active theme")?;
    let mut palette = resolve_palette(
        draft,
        draft.session_palette.as_deref().unwrap_or(active),
        appearance,
    )?;

    palette.apply_palette_fields(&draft.palette_overrides)?;
    palette.resolve_icon_variant(appearance.resolved(draft.terminal_appearance));
    if let Some(policy) = draft.session_file_color_policy {
        palette.apply_dynamic_file_color_policy(policy);
    }

    Ok(palette)
}

pub(super) fn effective_syntax(draft: &ConfigDraft) -> Result<SyntaxTheme, String> {
    let active = draft.active_theme.as_deref().ok_or("no active theme")?;
    let theme_syntax = theme_syntax_name(draft, active)?;
    let name = draft
        .session_syntax
        .as_ref()
        .and_then(|name| name.as_deref())
        .unwrap_or(&theme_syntax);
    resolve_syntax(draft, name)?.with_fields(&draft.syntax_overrides)
}

pub(super) fn effective_style(draft: &ConfigDraft) -> Result<DisplayStyle, String> {
    let style = resolve_style(draft, draft.session_style.as_deref().unwrap_or("modern"))?;
    DisplayStyle::from_fields(&draft.style_overrides, style)
}

fn effective_theme(draft: &ConfigDraft, appearance: TerminalAppearance) -> Result<Theme, String> {
    let mut theme = effective_palette(draft, appearance)?;
    effective_style(draft)?.apply_to_theme(&mut theme);

    Ok(theme)
}

fn available_presets(draft: &ConfigDraft, kind: &str) -> Vec<String> {
    let mut names = match kind {
        "theme" => draft.theme_specs.keys().cloned().collect::<Vec<_>>(),
        "syntax" => draft.syntax_themes.keys().cloned().collect(),
        "style" => draft.styles.keys().cloned().collect(),
        _ => Vec::new(),
    };

    names.retain(|name| !name.starts_with('@'));
    if matches!(kind, "theme" | "syntax") {
        names.push("terminal".into());
    }
    names.sort();
    names.dedup();

    names
}

pub(super) fn unknown_preset(draft: &ConfigDraft, kind: &str, name: &str) -> String {
    format!(
        "unknown {kind} preset '{name}'; available: {}",
        available_presets(draft, kind).join(", ")
    )
}

pub(super) fn select_theme(
    draft: &mut ConfigDraft,
    name: &str,
    appearance: TerminalAppearance,
) -> Result<(), String> {
    theme_spec(draft, name).map_err(|_| unknown_preset(draft, "theme", name))?;
    resolve_palette(draft, name, appearance)?;
    resolve_syntax(draft, &theme_syntax_name(draft, name)?)?;

    draft.active_theme = Some(name.to_owned());
    draft.session_palette = None;
    draft.session_syntax = None;
    draft.session_file_color_policy = None;
    draft.palette_overrides.clear();
    draft.syntax_overrides.clear();

    Ok(())
}

pub(super) fn select_palette(
    draft: &mut ConfigDraft,
    name: &str,
    appearance: TerminalAppearance,
) -> Result<(), String> {
    resolve_palette(draft, name, appearance).map_err(|error| {
        if error.starts_with("unknown UI theme:") {
            unknown_preset(draft, "theme", name)
        } else {
            error
        }
    })?;

    draft.session_palette = Some(name.into());
    draft.session_file_color_policy = None;
    draft.palette_overrides.clear();

    Ok(())
}

pub(super) fn select_syntax(draft: &mut ConfigDraft, name: &str) -> Result<(), String> {
    if name != "off" {
        resolve_syntax(draft, name).map_err(|error| {
            if error.starts_with("unknown syntax theme:") {
                unknown_preset(draft, "syntax", name)
            } else {
                error
            }
        })?;
    }

    draft.session_syntax = Some((name != "off").then(|| name.into()));
    draft.syntax_overrides.clear();

    Ok(())
}

pub(super) fn select_style(draft: &mut ConfigDraft, name: &str) -> Result<(), String> {
    resolve_style(draft, name).map_err(|error| {
        if error.starts_with("unknown style:") {
            unknown_preset(draft, "style", name)
        } else {
            error
        }
    })?;

    draft.session_style = Some(name.into());
    draft.style_overrides.clear();

    Ok(())
}

fn resolve_palette(
    draft: &ConfigDraft,
    name: &str,
    appearance: TerminalAppearance,
) -> Result<Theme, String> {
    fn resolve(
        draft: &ConfigDraft,
        name: &str,
        appearance: TerminalAppearance,
        seen: &mut HashSet<String>,
    ) -> Result<Theme, String> {
        if !seen.insert(name.into()) {
            return Err(format!("theme inheritance cycle at '{name}'"));
        }

        let result = if let Some(fields) = draft.themes.get(name) {
            if let Some(parent) = draft.palette_parents.get(name) {
                let mut theme = resolve(draft, parent, appearance, seen)?;
                theme.apply_palette_fields(fields)?;

                Ok(theme)
            } else {
                Theme::from_fields(fields)
            }
        } else if synthetic_theme_name(name) || name == "@generated_terminal" {
            Ok(derive_default_theme(
                appearance.resolved(draft.terminal_appearance),
            ))
        } else {
            Err(format!("unknown UI theme: {name}"))
        };
        seen.remove(name);

        result.map(|mut theme| {
            if draft.internal_nearest_palettes.contains(name)
                && let ratatui::style::Color::Rgb(red, green, blue) = theme.base
            {
                theme.use_nearest_file_palette(
                    crate::theme::FAR_PALETTE,
                    crate::theme::Rgb { red, green, blue },
                    false,
                );
            }

            theme
        })
    }

    let mut theme = resolve(draft, name, appearance, &mut HashSet::new())?;
    theme.resolve_icon_variant(appearance.resolved(draft.terminal_appearance));

    Ok(theme)
}

fn resolve_syntax(draft: &ConfigDraft, name: &str) -> Result<SyntaxTheme, String> {
    fn resolve(
        draft: &ConfigDraft,
        name: &str,
        seen: &mut HashSet<String>,
    ) -> Result<SyntaxTheme, String> {
        if !seen.insert(name.into()) {
            return Err(format!("syntax inheritance cycle at '{name}'"));
        }

        let result = if let Some(definition) = draft.syntax_themes.get(name) {
            if let Some(parent) = definition.parent.as_deref() {
                resolve(draft, parent, seen)?.with_fields(&definition.fields)
            } else {
                SyntaxTheme::from_fields(&definition.fields)
            }
        } else if name == "terminal" || name == "@generated_terminal" {
            Ok(SyntaxTheme::generated_default())
        } else {
            Err(format!("unknown syntax theme: {name}"))
        };
        seen.remove(name);

        result
    }

    resolve(draft, name, &mut HashSet::new())
}

fn resolve_style(draft: &ConfigDraft, name: &str) -> Result<DisplayStyle, String> {
    fn resolve(
        draft: &ConfigDraft,
        name: &str,
        seen: &mut HashSet<String>,
    ) -> Result<DisplayStyle, String> {
        if !seen.insert(name.into()) {
            return Err(format!("style inheritance cycle at '{name}'"));
        }

        let result = if let Some(definition) = draft.styles.get(name) {
            let base = match definition.parent.as_deref() {
                Some(parent) => resolve(draft, parent, seen)?,
                None => DisplayStyle::default(),
            };
            DisplayStyle::from_fields(&definition.fields, base)
        } else {
            Err(format!("unknown style: {name}"))
        };
        seen.remove(name);

        result
    }

    resolve(draft, name, &mut HashSet::new())
}

fn palette_is_generated(draft: &ConfigDraft, name: &str) -> Result<bool, String> {
    let mut current = name;
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(current) {
            return Err(format!("theme inheritance cycle at '{current}'"));
        }

        if draft.themes.contains_key(current) {
            if let Some(parent) = draft.palette_parents.get(current) {
                current = parent;
                continue;
            }

            return Ok(false);
        }

        return Ok(synthetic_theme_name(current) || current == "@generated_terminal");
    }
}
