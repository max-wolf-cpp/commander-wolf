use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, Table, Value};

use super::ConfigDraft;
use super::appearance::{
    ComponentDefinition, ThemeSpec, effective_palette, effective_style, effective_syntax,
    select_style, select_syntax, select_theme, unknown_preset,
};
use crate::display_style::DisplayStyle;
use crate::icons::IconMode;
use crate::syntax::SyntaxTheme;
use crate::terminal_appearance::TerminalAppearance;
use crate::theme::{SeparatorStyle, Theme};

#[derive(Clone, Copy)]
pub(super) enum ThemePart {
    Theme,
    Syntax,
    Style,
}

impl ThemePart {
    fn name(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Syntax => "syntax",
            Self::Style => "style",
        }
    }
}

pub(super) fn component_api(
    lua: &Lua,
    draft: Rc<RefCell<ConfigDraft>>,
    appearance: Rc<Cell<TerminalAppearance>>,
    part: ThemePart,
    prefix: String,
) -> mlua::Result<Table> {
    let proxy = lua.create_table()?;
    let methods = component_methods(
        lua,
        Rc::clone(&draft),
        Rc::clone(&appearance),
        part,
        &prefix,
    )?;

    let meta = lua.create_table()?;
    let read_draft = Rc::clone(&draft);
    let read_appearance = Rc::clone(&appearance);
    let read_prefix = prefix.clone();
    meta.set(
        "__index",
        lua.create_function(move |lua, (_, key): (Table, String)| {
            if read_prefix.is_empty() && matches!(key.as_str(), "use" | "define") {
                return methods.raw_get::<Value>(key);
            }

            if matches!(part, ThemePart::Style)
                && read_prefix.is_empty()
                && matches!(key.as_str(), "panels" | "footer" | "frames")
            {
                return Ok(Value::Table(component_api(
                    lua,
                    Rc::clone(&read_draft),
                    Rc::clone(&read_appearance),
                    part,
                    format!("{key}."),
                )?));
            }

            let name = format!("{read_prefix}{key}");
            read_component_property(
                lua,
                &read_draft.borrow(),
                read_appearance.get(),
                part,
                &name,
            )
        })?,
    )?;
    meta.set(
        "__newindex",
        lua.create_function(move |_, (_, key, value): (Table, String, Value)| {
            let name = format!("{prefix}{key}");
            write_component_property(
                &mut draft.borrow_mut(),
                appearance.get(),
                part,
                &name,
                value,
            )
        })?,
    )?;
    proxy.set_metatable(Some(meta))?;

    Ok(proxy)
}

fn component_methods(
    lua: &Lua,
    draft: Rc<RefCell<ConfigDraft>>,
    appearance: Rc<Cell<TerminalAppearance>>,
    part: ThemePart,
    prefix: &str,
) -> mlua::Result<Table> {
    let methods = lua.create_table()?;
    if prefix.is_empty() {
        let define_draft = Rc::clone(&draft);
        methods.set(
            "define",
            lua.create_function(move |lua, (name, values): (String, Table)| {
                if matches!(part, ThemePart::Theme) {
                    define_theme_preset(lua, &mut define_draft.borrow_mut(), &name, values)
                } else {
                    validate_registry_name(&name).map_err(mlua::Error::runtime)?;
                    let definition = component_definition(values)?;
                    validate_component_definition(part, &definition)?;
                    register_component_definition(
                        &mut define_draft.borrow_mut(),
                        part,
                        name,
                        definition,
                    );
                    Ok(())
                }
            })?,
        )?;
        let use_draft = Rc::clone(&draft);
        let use_appearance = Rc::clone(&appearance);
        methods.set(
            "use",
            lua.create_function(move |_, name: String| {
                let mut draft = use_draft.borrow_mut();
                if matches!(part, ThemePart::Syntax) && name == "off" {
                    return Err(mlua::Error::runtime(unknown_preset(
                        &draft, "syntax", &name,
                    )));
                }

                let result = match part {
                    ThemePart::Theme => select_theme(&mut draft, &name, use_appearance.get()),
                    ThemePart::Syntax => select_syntax(&mut draft, &name),
                    ThemePart::Style => select_style(&mut draft, &name),
                };
                result.map_err(mlua::Error::runtime)
            })?,
        )?;
    }

    Ok(methods)
}

fn define_theme_preset(
    lua: &Lua,
    draft: &mut ConfigDraft,
    name: &str,
    values: Table,
) -> mlua::Result<()> {
    validate_registry_name(name).map_err(mlua::Error::runtime)?;

    let theme_fields = lua.create_table()?;
    let mut syntax = None;
    let mut inline_syntax = None;
    for pair in values.pairs::<String, Value>() {
        let (key, value) = pair?;
        if key == "syntax" {
            syntax = Some(match value {
                Value::String(value) => value.to_str()?.to_owned(),
                Value::Table(table) => {
                    let definition = component_definition(table)?;
                    validate_component_definition(ThemePart::Syntax, &definition)?;
                    let generated = format!("@theme.{name}.syntax");
                    inline_syntax = Some((generated.clone(), definition));
                    generated
                }
                _ => {
                    return Err(mlua::Error::runtime(format!(
                        "theme '{name}' syntax must name or define a syntax preset"
                    )));
                }
            });
        } else {
            theme_fields.set(key, value)?;
        }
    }

    let definition = component_definition(theme_fields)?;
    validate_component_definition(ThemePart::Theme, &definition)?;
    if definition.parent.is_none() && syntax.is_none() {
        return Err(mlua::Error::runtime(format!(
            "theme '{name}' must define syntax or extend another theme"
        )));
    }

    let generated = format!("@theme.{name}.syntax");
    draft.syntax_themes.remove(&generated);
    if let Some((generated, definition)) = inline_syntax {
        register_component_definition(draft, ThemePart::Syntax, generated, definition);
    }
    register_component_definition(draft, ThemePart::Theme, name.into(), definition);
    draft.theme_specs.insert(name.into(), ThemeSpec { syntax });

    Ok(())
}

fn validate_component_definition(
    part: ThemePart,
    definition: &ComponentDefinition,
) -> mlua::Result<()> {
    match part {
        ThemePart::Theme => {
            Theme::emergency_monochrome()
                .apply_palette_fields(&definition.fields)
                .map_err(mlua::Error::runtime)?;
            if definition.parent.is_none() {
                Theme::from_fields(&definition.fields).map_err(mlua::Error::runtime)?;
            }
        }
        ThemePart::Syntax => {
            SyntaxTheme::generated_default()
                .with_fields(&definition.fields)
                .map_err(mlua::Error::runtime)?;
            if definition.parent.is_none() {
                SyntaxTheme::from_fields(&definition.fields).map_err(mlua::Error::runtime)?;
            }
        }
        ThemePart::Style => {
            DisplayStyle::from_fields(&definition.fields, DisplayStyle::default())
                .map_err(mlua::Error::runtime)?;
        }
    }

    Ok(())
}

fn register_component_definition(
    draft: &mut ConfigDraft,
    part: ThemePart,
    name: String,
    definition: ComponentDefinition,
) {
    match part {
        ThemePart::Theme => {
            draft.internal_nearest_palettes.remove(&name);
            if let Some(parent) = definition.parent {
                draft.palette_parents.insert(name.clone(), parent);
            } else {
                draft.palette_parents.remove(&name);
            }
            draft.themes.insert(name, definition.fields);
        }
        ThemePart::Syntax => {
            draft.syntax_themes.insert(name, definition);
        }
        ThemePart::Style => {
            draft.styles.insert(name, definition);
        }
    }
}

fn read_component_property(
    lua: &Lua,
    draft: &ConfigDraft,
    appearance: TerminalAppearance,
    part: ThemePart,
    name: &str,
) -> mlua::Result<Value> {
    match part {
        ThemePart::Theme => effective_palette(draft, appearance)
            .map_err(mlua::Error::runtime)?
            .palette_field(name)
            .map_err(mlua::Error::runtime)?
            .map(|value| lua.create_string(value).map(Value::String))
            .transpose()
            .map(|value| value.unwrap_or(Value::Nil)),
        ThemePart::Syntax => {
            let category = crate::syntax::SyntaxCategory::ALL
                .into_iter()
                .find(|category| category.name() == name)
                .ok_or_else(|| mlua::Error::runtime(format!("unknown syntax property '{name}'")))?;
            let color = effective_syntax(draft)
                .map_err(mlua::Error::runtime)?
                .color(category, ratatui::style::Color::Reset);
            Ok(Value::String(
                lua.create_string(crate::theme::format_theme_color(color))?,
            ))
        }
        ThemePart::Style => style_property(
            lua,
            &effective_style(draft).map_err(mlua::Error::runtime)?,
            name,
        ),
    }
}

fn style_property(lua: &Lua, style: &DisplayStyle, name: &str) -> mlua::Result<Value> {
    let boolean = match name {
        "color_file_names" => Some(style.color_file_names),
        "title_bold" => Some(style.title_bold),
        "status_bold" => Some(style.status_bold),
        "dim_secondary" => Some(style.dim_secondary),
        "framed_footer_border_follows_mode" => Some(style.framed_footer_border_follows_mode),
        _ => None,
    };
    if let Some(boolean) = boolean {
        return Ok(Value::Boolean(boolean));
    }

    let value = match name {
        "panels.layout" => style.panels.name(),
        "footer.layout" => style.footer.name(),
        "frames.border" => style.frames.name(),
        "panels.border" => style.panel_border.name(),
        "footer.border" => style.footer_border.name(),
        "separator" => match style.separator {
            SeparatorStyle::Thin => "plain",
            SeparatorStyle::Rounded => "rounded",
            SeparatorStyle::Double => "double",
            SeparatorStyle::Thick => "thick",
            SeparatorStyle::Simple => "simple",
            SeparatorStyle::Dashed => "dashed",
            SeparatorStyle::Off => "off",
        },
        "decorations" => {
            if style.decorations {
                "nerd_font"
            } else {
                "none"
            }
        }
        "icons" => match style.icons {
            IconMode::NerdFont => "nerd_font",
            IconMode::Unicode => "unicode",
            IconMode::None => "none",
        },
        "panels.title_left" => &style.panel_title_left,
        "panels.title_right" => &style.panel_title_right,
        "footer.separator_left" => &style.footer_separator_left,
        "footer.separator_right" => &style.footer_separator_right,
        _ => {
            return Err(mlua::Error::runtime(format!(
                "unknown style property '{name}'"
            )));
        }
    };

    Ok(Value::String(lua.create_string(value)?))
}

fn write_component_property(
    draft: &mut ConfigDraft,
    appearance: TerminalAppearance,
    part: ThemePart,
    name: &str,
    value: Value,
) -> mlua::Result<()> {
    validate_property_name(part, name)?;

    let boolean_style = matches!(
        name,
        "color_file_names"
            | "title_bold"
            | "status_bold"
            | "dim_secondary"
            | "framed_footer_border_follows_mode"
    );
    let value = match (boolean_style && matches!(part, ThemePart::Style), value) {
        (true, Value::Boolean(value)) => value.to_string(),
        (false, Value::String(value)) => value.to_str()?.to_owned(),
        _ => {
            return Err(mlua::Error::runtime(format!(
                "{} property '{name}' has an invalid value type",
                part.name()
            )));
        }
    };

    let patch = HashMap::from([(name.to_owned(), value.clone())]);
    match part {
        ThemePart::Theme => {
            effective_palette(draft, appearance)
                .map_err(mlua::Error::runtime)?
                .apply_palette_fields(&patch)
                .map_err(mlua::Error::runtime)?;
            draft.palette_overrides.insert(name.into(), value);
        }
        ThemePart::Syntax => {
            effective_syntax(draft)
                .map_err(mlua::Error::runtime)?
                .with_fields(&patch)
                .map_err(mlua::Error::runtime)?;
            draft.syntax_overrides.insert(name.into(), value);
        }
        ThemePart::Style => {
            DisplayStyle::from_fields(
                &patch,
                effective_style(draft).map_err(mlua::Error::runtime)?,
            )
            .map_err(mlua::Error::runtime)?;
            draft.style_overrides.insert(name.into(), value);
        }
    }

    Ok(())
}

fn validate_property_name(part: ThemePart, name: &str) -> mlua::Result<()> {
    match part {
        ThemePart::Theme => {
            Theme::emergency_monochrome()
                .palette_field(name)
                .map_err(mlua::Error::runtime)?;
        }
        ThemePart::Syntax => {
            if !crate::syntax::SyntaxCategory::ALL
                .iter()
                .any(|category| category.name() == name)
            {
                return Err(mlua::Error::runtime(format!(
                    "unknown syntax property '{name}'"
                )));
            }
        }
        ThemePart::Style => {
            if !matches!(
                name,
                "panels.layout"
                    | "footer.layout"
                    | "frames.border"
                    | "panels.border"
                    | "footer.border"
                    | "separator"
                    | "decorations"
                    | "icons"
                    | "color_file_names"
                    | "title_bold"
                    | "status_bold"
                    | "dim_secondary"
                    | "framed_footer_border_follows_mode"
                    | "panels.title_left"
                    | "panels.title_right"
                    | "footer.separator_left"
                    | "footer.separator_right"
            ) {
                return Err(mlua::Error::runtime(format!(
                    "unknown style property '{name}'"
                )));
            }
        }
    }

    Ok(())
}

fn component_definition(values: Table) -> mlua::Result<ComponentDefinition> {
    let parent: Option<String> = values.get("extends")?;
    if let Some(parent) = parent.as_deref() {
        validate_registry_name(parent).map_err(mlua::Error::runtime)?;
    }

    let mut fields = HashMap::new();
    collect_fields(values, "", &mut fields, true)?;

    Ok(ComponentDefinition { parent, fields })
}

fn validate_registry_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "off"
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        return Err(format!("invalid theme component name '{name}'"));
    }

    Ok(())
}

fn collect_fields(
    values: Table,
    prefix: &str,
    fields: &mut HashMap<String, String>,
    skip_extends: bool,
) -> mlua::Result<()> {
    for pair in values.pairs::<String, Value>() {
        let (key, value) = pair?;
        if skip_extends && prefix.is_empty() && key == "extends" {
            continue;
        }

        let name = if prefix.is_empty() {
            key
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(child) => collect_fields(child, &name, fields, false)?,
            Value::String(value) => {
                fields.insert(name, value.to_str()?.to_owned());
            }
            Value::Boolean(value) => {
                fields.insert(name, value.to_string());
            }
            _ => {
                return Err(mlua::Error::runtime(format!(
                    "field '{name}' must be a string, boolean, or table"
                )));
            }
        }
    }

    Ok(())
}
