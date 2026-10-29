use std::path::PathBuf;

use crate::app::{Action, Notification};
use crate::file_opening::Application;
use crate::theme::Theme;

use super::super::media::MediaRuntime;
use super::super::terminal_writer::TerminalWriter;

pub(super) fn open_file(
    path: PathBuf,
    application: Option<Application>,
    text_editor: Option<Application>,
    media: &mut MediaRuntime,
    writer: &mut TerminalWriter,
    theme: &Theme,
) -> Result<Vec<Action>, String> {
    if let Some(application) = application.as_ref() {
        return crate::file_opening::open_external(&path, Some(application)).map(|()| Vec::new());
    }

    let Some(editor) = text_editor
        .as_ref()
        .filter(|_| crate::filesystem::text_preview::is_text(&path))
    else {
        return crate::file_opening::open_external(&path, None).map(|()| Vec::new());
    };

    media.clear_current(theme, writer)?;
    writer.suspend();
    let editor_result = crate::file_opening::run_text_editor(&path, editor);
    writer.resume();

    let mut actions = vec![Action::ExternalEditorClosed];
    if let Err(editor_error) = editor_result
        && let Err(default_error) = crate::file_opening::open_external(&path, None)
    {
        actions.push(Action::Notify(Notification::error(format!(
            "{editor_error}; operating system fallback also failed: {default_error}"
        ))));
    }

    Ok(actions)
}

pub(super) fn open_editor_in_directory(
    directory: PathBuf,
    editor: Application,
    media: &mut MediaRuntime,
    writer: &mut TerminalWriter,
    theme: &Theme,
) -> Result<Vec<Action>, String> {
    media.clear_current(theme, writer)?;
    writer.suspend();
    let result = crate::file_opening::run_editor_in_directory(&directory, &editor);
    writer.resume();

    let mut actions = vec![Action::ExternalEditorClosed];
    if let Err(error) = result {
        actions.push(Action::Notify(Notification::error(error)));
    }

    Ok(actions)
}
