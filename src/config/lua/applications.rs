use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Table, Value};

use super::{ConfigDraft, ConfigRuntime, lua_error};
use crate::file_opening::{Application, FileAssociations};

impl ConfigRuntime {
    pub fn file_associations(&self) -> FileAssociations {
        self.draft.borrow().file_associations.clone()
    }

    pub fn shell(&self) -> Option<Application> {
        self.draft.borrow().shell.clone()
    }
}

pub(super) fn file_association_api(
    lua: &Lua,
    draft: Rc<RefCell<ConfigDraft>>,
) -> Result<Table, String> {
    let api = lua.create_table().map_err(lua_error)?;
    let set_draft = Rc::clone(&draft);
    api.set(
        "set",
        lua.create_function(move |_, (extensions, application): (Value, Value)| {
            let extensions = file_association_extensions(extensions)?;
            let application = parse_application(application)?;
            set_draft
                .borrow_mut()
                .file_associations
                .set(extensions, application)
                .map_err(mlua::Error::runtime)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let delete_draft = Rc::clone(&draft);
    api.set(
        "del",
        lua.create_function(move |_, extensions: Value| {
            delete_draft
                .borrow_mut()
                .file_associations
                .remove(file_association_extensions(extensions)?)
                .map_err(mlua::Error::runtime)
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let editor_draft = Rc::clone(&draft);
    api.set(
        "set_text_editor",
        lua.create_function(move |_, application: Value| {
            editor_draft
                .borrow_mut()
                .file_associations
                .set_text_editor(parse_application(application)?);
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    let clear_editor_draft = Rc::clone(&draft);
    api.set(
        "clear_text_editor",
        lua.create_function(move |_, ()| {
            clear_editor_draft
                .borrow_mut()
                .file_associations
                .clear_text_editor();
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    api.set(
        "clear",
        lua.create_function(move |_, ()| {
            draft.borrow_mut().file_associations.clear();
            Ok(())
        })
        .map_err(lua_error)?,
    )
    .map_err(lua_error)?;

    Ok(api)
}

fn file_association_extensions(value: Value) -> mlua::Result<Vec<String>> {
    match value {
        Value::String(extension) => Ok(vec![extension.to_str()?.to_owned()]),
        Value::Table(extensions) => {
            let extensions = extensions
                .sequence_values::<String>()
                .collect::<mlua::Result<Vec<_>>>()?;
            if extensions.is_empty() {
                return Err(mlua::Error::runtime(
                    "file association extension list must not be empty",
                ));
            }

            Ok(extensions)
        }
        _ => Err(mlua::Error::runtime(
            "file association extensions must be a string or an array of strings",
        )),
    }
}

pub(super) fn parse_application(value: Value) -> mlua::Result<Application> {
    let (program, arguments) = match value {
        Value::String(program) => (program.to_str()?.to_owned(), Vec::new()),
        Value::Table(application) => {
            let program = application.get::<String>("program").map_err(|_| {
                mlua::Error::runtime("file association application requires string 'program'")
            })?;
            let arguments = application
                .get::<Option<Table>>("args")?
                .map(|arguments| {
                    arguments
                        .sequence_values::<String>()
                        .collect::<mlua::Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            (program, arguments)
        }
        _ => {
            return Err(mlua::Error::runtime(
                "file association application must be a program string or a table",
            ));
        }
    };

    Application::new(program, arguments).map_err(mlua::Error::runtime)
}
