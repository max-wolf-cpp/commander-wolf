use super::*;
use crate::file_operations::{FileOperation, PanelRefreshRequest};

#[test]
fn unicode_editing_uses_character_columns() {
    let mut edit = DirectoryEditState {
        active: true,
        lines: vec![EditLine {
            text: "狼".into(),
            original: None,
        }],
        ..Default::default()
    };
    edit.cursor_column = 1;
    edit.insert('犬');
    assert_eq!(edit.lines[0].text, "狼犬");
    edit.backspace();
    assert_eq!(edit.lines[0].text, "狼");
}

#[test]
fn diff_maps_renames_deletes_files_and_slash_directories() {
    let mut edit = DirectoryEditState::default();
    edit.begin(
        PanelId::Left,
        "root".into(),
        &[
            DirectoryEntry {
                path: "root/a".into(),
                name: "a".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            },
            DirectoryEntry {
                path: "root/dir".into(),
                name: "dir".into(),
                is_directory: true,
                is_parent: false,
                is_hidden: false,
            },
        ],
        0,
    );
    edit.lines[0].text = "renamed".into();
    edit.lines.remove(1);
    edit.lines.push(EditLine {
        text: "new/".into(),
        original: None,
    });
    let request = edit
        .request(
            "protected".into(),
            [
                PanelRefreshRequest {
                    panel: PanelId::Left,
                    directory: "root".into(),
                    selection: crate::file_operations::RefreshSelection {
                        preferred: None,
                        fallback_index: 0,
                    },
                },
                PanelRefreshRequest {
                    panel: PanelId::Right,
                    directory: "other".into(),
                    selection: crate::file_operations::RefreshSelection {
                        preferred: None,
                        fallback_index: 0,
                    },
                },
            ],
        )
        .unwrap();
    assert!(matches!(
        request.operations[0],
        FileOperation::Delete { .. }
    ));
    assert!(
        request
            .operations
            .iter()
            .any(|op| matches!(op, FileOperation::Rename { .. }))
    );
    assert!(
        request
            .operations
            .iter()
            .any(|op| matches!(op, FileOperation::CreateDirectory { .. }))
    );
    assert_eq!(
        edit.changes(),
        vec![
            DirectoryEditChange::Create { name: "new".into() },
            DirectoryEditChange::Rename {
                source: "a".into(),
                destination: "renamed".into(),
            },
            DirectoryEditChange::Delete { name: "dir".into() },
        ]
    );
}

#[test]
fn snapshot_comparison_ignores_empty_new_lines_and_directory_markers() {
    let mut edit = DirectoryEditState::default();
    edit.begin(
        PanelId::Left,
        "root".into(),
        &[DirectoryEntry {
            path: "root/dir".into(),
            name: "dir".into(),
            is_directory: true,
            is_parent: false,
            is_hidden: false,
        }],
        0,
    );
    assert!(!edit.has_changes());
    edit.lines[0].text.push('/');
    edit.lines.push(EditLine {
        text: String::new(),
        original: None,
    });
    assert!(!edit.has_changes());
    edit.lines.last_mut().unwrap().text = "new.txt".into();
    assert!(edit.has_changes());
}
