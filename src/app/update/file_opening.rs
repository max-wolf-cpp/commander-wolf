use super::playback::stop_video;
use super::preview::prepare_selected;
use super::viewer::open_viewer;
use super::workspace::open_selected_directory;
use crate::app::{Action, AppState, Effect, Notification};

pub(super) fn update(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::OpenSelectedFile => {
            let Some(entry) = state.workspace.active().selected_entry().cloned() else {
                return Vec::new();
            };

            if entry.is_directory {
                return open_selected_directory(state, entry);
            }

            let application = state
                .file_associations
                .application_for(&entry.path)
                .cloned();
            if application.is_none()
                && state.preview_options.use_internal_image_viewer
                && crate::file_opening::uses_internal_image_viewer(&entry.path)
            {
                return open_viewer(state, entry.path, entry.name);
            }

            let mut effects = stop_video(state);
            effects.push(Effect::OpenExternalFile {
                path: entry.path,
                application,
                text_editor: state.file_associations.text_editor().cloned(),
            });
            effects
        }
        Action::RevealSelectedInFileManager => state
            .workspace
            .active()
            .selected_entry()
            .map(|entry| Effect::RevealInFileManager {
                path: entry.path.clone(),
                is_directory: entry.is_directory,
            })
            .into_iter()
            .collect(),
        Action::OpenEditorInActiveDirectory(editor) => {
            let editor = editor.or_else(|| state.file_associations.text_editor().cloned());
            let Some(editor) = editor else {
                state.notification = Some(Notification::error(
                    "No text editor configured; use :editor <program> or wolf.file_association.set_text_editor()",
                ));
                return Vec::new();
            };

            state.notification = None;
            vec![Effect::OpenEditorInDirectory {
                directory: state.workspace.active().directory.clone(),
                editor,
            }]
        }
        Action::ExternalEditorClosed => prepare_selected(state),
        _ => unreachable!("dispatcher routed an unsupported action to file_opening"),
    }
}
