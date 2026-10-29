use std::cell::RefCell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use mlua::{Lua, Table, Value};

use super::appearance::effective_style;
use super::applications::parse_application;
use super::{ConfigDraft, ConfigRuntime, lua_error};
use crate::dialogs::{DialogDisplay, DialogOptions};
use crate::finder::{FinderOptions, MAX_CONFIGURED_RESULTS};
use crate::preview::PreviewOptions;
use crate::status_line::{ModePosition, StatusLineOptions};
use crate::terminal_appearance::SchemePreference;
use crate::workspace::{LineNumberMode, WorkspaceOptions};

impl ConfigRuntime {
    pub fn workspace_options(&self) -> WorkspaceOptions {
        let draft = self.draft.borrow();
        let mut options = draft.workspace;
        if let Ok(style) = effective_style(&draft) {
            options.icon_mode = style.icons;
            options.color_file_names = style.color_file_names;
            options.panel_style = style.panels;
        }

        options
    }

    pub fn finder_options(&self) -> FinderOptions {
        self.draft.borrow().finder
    }

    pub fn preview_options(&self) -> PreviewOptions {
        self.draft.borrow().preview
    }

    pub fn status_line_options(&self) -> StatusLineOptions {
        let draft = self.draft.borrow();
        let mut options = draft.status_line;
        if let Ok(style) = effective_style(&draft) {
            options.style = style.footer;
        }

        options
    }

    pub fn dialog_options(&self) -> DialogOptions {
        self.draft.borrow().dialogs
    }

    pub fn remember_directories(&self) -> bool {
        self.draft.borrow().remember_directories
    }
}

pub(super) fn option_api(lua: &Lua, draft: Rc<RefCell<ConfigDraft>>) -> Result<Table, String> {
    let api = lua.create_table().map_err(lua_error)?;
    api.set(
        "set",
        lua.create_function(move |_, (name, value): (String, Value)| {
            set_option(&mut draft.borrow_mut(), &name, value)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    Ok(api)
}

fn set_option(draft: &mut ConfigDraft, name: &str, value: Value) -> mlua::Result<()> {
    match name {
        "terminal.appearance" => {
            let Value::String(value) = value else {
                return Err(mlua::Error::runtime(
                    "terminal.appearance must be 'auto', 'dark', or 'light'",
                ));
            };

            draft.terminal_appearance =
                SchemePreference::parse(value.to_str()?.as_ref()).map_err(mlua::Error::runtime)?;
        }
        "shell" => {
            draft.shell = match value {
                Value::Nil => None,
                Value::Table(_) => Some(parse_application(value)?),
                _ => {
                    return Err(mlua::Error::runtime(
                        "shell must be a { program, args } table or nil",
                    ));
                }
            };
        }
        "session.remember_directories" => draft.remember_directories = boolean_option(name, value)?,
        "workspace.show_hidden" => draft.workspace.show_hidden = boolean_option(name, value)?,
        "workspace.line_numbers" => {
            let Value::String(value) = value else {
                return Err(mlua::Error::runtime(
                    "workspace.line_numbers must be 'off', 'absolute', or 'relative'",
                ));
            };

            draft.workspace.line_numbers =
                LineNumberMode::parse(value.to_str()?.as_ref()).map_err(mlua::Error::runtime)?;
        }
        "workspace.scroll_offset" => {
            draft.workspace.scroll_offset = non_negative_option(name, value)?
        }
        "finder.show_hidden" => draft.finder.show_hidden = boolean_option(name, value)?,
        "finder.scroll_offset" => draft.finder.scroll_offset = non_negative_option(name, value)?,
        "finder.max_results" => {
            draft.finder.max_results =
                bounded_option(name, value, 1..=MAX_CONFIGURED_RESULTS as u64)? as usize
        }
        "preview.media_percent" => {
            draft.preview.media_percent = bounded_option(name, value, 20..=80)? as u16
        }
        "preview.documents.enabled" => {
            draft.preview.documents.enabled = boolean_option(name, value)?
        }
        "preview.documents.cache_pages" => {
            draft.preview.documents.cache_pages = bounded_option(name, value, 3..=32)? as usize
        }
        "preview.documents.max_render_pixels" => {
            draft.preview.documents.max_render_pixels =
                bounded_option(name, value, 65_536..=64_000_000)?
        }
        "viewer.max_percent" => {
            draft.preview.viewer_max_percent = bounded_option(name, value, 20..=100)? as u16
        }
        "viewer.use_internal" => {
            draft.preview.use_internal_image_viewer = boolean_option(name, value)?
        }
        "ui.dialogs" => {
            let Value::String(value) = value else {
                return Err(mlua::Error::runtime(
                    "ui.dialogs must be 'float' or 'command_line'",
                ));
            };

            draft.dialogs.display = DialogDisplay::parse(value.to_str()?.as_ref())
                .map_err(|error| mlua::Error::runtime(format!("ui.dialogs: {error}")))?;
        }
        "status.mode_position" => {
            let Value::String(value) = value else {
                return Err(mlua::Error::runtime(
                    "status.mode_position must be 'left' or 'right'",
                ));
            };

            draft.status_line.mode_position =
                ModePosition::parse(value.to_str()?.as_ref()).map_err(mlua::Error::runtime)?;
        }
        _ => {
            return Err(mlua::Error::runtime(format!(
                "unknown Commander Wolf option: {name}"
            )));
        }
    }

    Ok(())
}

fn boolean_option(name: &str, value: Value) -> mlua::Result<bool> {
    match value {
        Value::Boolean(value) => Ok(value),
        _ => Err(mlua::Error::runtime(format!("{name} must be a boolean"))),
    }
}

fn non_negative_option(name: &str, value: Value) -> mlua::Result<usize> {
    let error = || mlua::Error::runtime(format!("{name} must be a non-negative integer"));
    let Value::Integer(value) = value else {
        return Err(error());
    };

    usize::try_from(value).map_err(|_| error())
}

fn bounded_option(name: &str, value: Value, range: RangeInclusive<u64>) -> mlua::Result<u64> {
    let error = || {
        mlua::Error::runtime(format!(
            "{name} must be an integer from {} to {}",
            range.start(),
            range.end()
        ))
    };
    let Value::Integer(value) = value else {
        return Err(error());
    };

    let value = u64::try_from(value).map_err(|_| error())?;
    if !range.contains(&value) {
        return Err(error());
    }

    Ok(value)
}
