use super::*;
use crate::dialogs::DialogKind;
use crate::dialogs::{FileConflict, FileConflictState};
use crate::directory_edit::EditLine;
use crate::file_operations::FileOperation;
use crate::file_operations::{FileOperationBatchRequest, PanelRefreshRequest, RefreshSelection};
use crate::preview::FileMetadata;
use crate::terminal::TerminalSize;
use crate::theme::Rgb;
use crate::workspace::{DirectoryEntry, PanelId, WorkspaceOptions};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::text::Line;

#[test]
fn file_conflict_surfaces_operation_title_actions_and_accented_shortcuts() {
    let mut state = conflict_fixture();

    let text = command_line_text(&state);
    assert!(text.starts_with("Copy conflict: File already exists | to/a | New"));
    assert!(text.contains("Existing"));
    assert!(text.contains("[O]verwrite   [S]kip   [C]ancel"));
    assert!(text.contains("[ ] Apply to [a]ll remaining conflicts"));
    state.file_conflict.as_mut().unwrap().toggle_apply_to_all();
    assert!(command_line_text(&state).contains("[x] Apply to [a]ll remaining conflicts"));

    let accent = ratatui::style::Color::Rgb(250, 179, 135);
    let mut theme = Theme::emergency_monochrome();
    theme.dialog_warning = Rgb {
        red: 250,
        green: 179,
        blue: 135,
    }
    .into();
    let (_, lines, height, _) = content(&state, &theme);
    assert_eq!(height, 11);
    assert_eq!(lines[6].spans[1].content, "a");
    for span in [
        &lines[6].spans[1],
        &lines[8].spans[1],
        &lines[8].spans[3],
        &lines[8].spans[5],
    ] {
        assert_eq!(span.style.fg, Some(accent));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }
    for span in [
        &lines[6].spans[0],
        &lines[6].spans[2],
        &lines[8].spans[0],
        &lines[8].spans[2],
        &lines[8].spans[4],
        &lines[8].spans[6],
    ] {
        assert_eq!(span.style, Style::default());
    }
    let buttons = lines[8]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert_eq!(buttons, " Overwrite   Skip   Cancel");

    state.mode = AppMode::Confirm;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| draw(frame, &state, &theme)).unwrap();
    let dialog = super::content::build(&state, &theme, false);
    let body = Paragraph::new(dialog.body).wrap(Wrap { trim: false });
    let actions = Paragraph::new(dialog.actions).wrap(Wrap { trim: false });
    let area = centered_dialog(Rect::new(0, 0, 80, 24), 76, &body, &actions);
    assert_eq!(
        terminal.backend().buffer()[(area.x, area.y)].fg,
        theme.dialog_warning.into()
    );
    let title_cell = &terminal.backend().buffer()[(area.x + 5, area.y)];
    assert_eq!(title_cell.symbol(), "C");
    assert_eq!(title_cell.fg, theme.dialog_warning.into());
    assert!(title_cell.modifier.contains(Modifier::BOLD));
    let inner_x = area.x + 1;
    let message_y = area.y + 1;
    assert_eq!(
        terminal.backend().buffer()[(inner_x, message_y)].symbol(),
        " "
    );
    assert_eq!(
        terminal.backend().buffer()[(inner_x + 1, message_y)].symbol(),
        "F"
    );
    let destination_y = area.y + 2;
    assert_eq!(
        terminal.backend().buffer()[(inner_x, destination_y)].symbol(),
        " "
    );
    assert_eq!(
        terminal.backend().buffer()[(inner_x + 1, destination_y)].symbol(),
        "t"
    );
    let actions_y = area.y + 9;
    assert_eq!(
        terminal.backend().buffer()[(inner_x, actions_y)].symbol(),
        " "
    );
    assert_eq!(
        terminal.backend().buffer()[(inner_x + 1, actions_y)].symbol(),
        "O"
    );

    let conflict_state = state.file_conflict.as_mut().unwrap();
    conflict_state.request.operations.truncate(1);
    let (_, lines, height, _) = content(&state, &theme);
    assert_eq!(height, 9);
    assert!(!command_line_text(&state).contains("Apply to"));
    assert!(lines.iter().all(|line| {
        !line
            .spans
            .iter()
            .any(|span| span.content.contains("Apply to"))
    }));

    let conflict_state = state.file_conflict.as_mut().unwrap();
    conflict_state.request.operations = vec![FileOperation::Move {
        source: "a".into(),
        destination: "to/a".into(),
        overwrite: false,
    }];
    assert_eq!(content(&state, &theme).0, "Move conflict");
    assert!(command_line_text(&state).starts_with("Move conflict"));
}

#[test]
fn conflict_dialog_fits_wrapped_paths_metadata_and_all_controls() {
    let paths = [
        r"E:\Repositories\MaxWolf\ConfigVault\WindowsTerminal\Microsoft.PowerShell_profile.ps1"
            .to_owned(),
        format!(
            "E:/{}長い名前/профиль с пробелами.txt",
            "long-directory/".repeat(18)
        ),
    ];
    for path in paths {
        for columns in [40, 80, 120] {
            for batch in [false, true] {
                let mut state = conflict_fixture();
                state.mode = AppMode::Confirm;
                let conflict_state = state.file_conflict.as_mut().unwrap();
                if !batch {
                    conflict_state.request.operations.truncate(1);
                }
                let conflict = &mut conflict_state.conflicts[0];
                conflict.destination = path.clone().into();
                conflict.new_metadata.size = "1.58 KiB (1,621 bytes)".into();
                conflict.new_metadata.modified = "2026-10-11 19:01:35 UTC".into();
                conflict.existing_metadata.modified = "2026-05-21 17:10:33 UTC".into();

                let text = rendered_dialog(&state, columns, 40);
                assert!(
                    text.contains("File already exists"),
                    "{columns}, {batch}: {text}"
                );
                assert!(text.contains("Existing"), "{columns}, {batch}: {text}");
                assert!(
                    text.contains("Overwrite   Skip   Cancel"),
                    "{columns}, {batch}: {text}"
                );
                if batch {
                    assert!(text.contains("Apply to all"), "{columns}: {text}");
                    assert!(text.contains("remaining"), "{columns}: {text}");
                    assert!(text.contains("conflicts"), "{columns}: {text}");
                }
                if columns == 120 && path.is_ascii() {
                    assert!(text.contains(&path), "{columns}: {text}");
                }
            }
        }
    }
}

#[test]
fn constrained_dialog_keeps_choices_visible_ahead_of_long_details() {
    for rows in [6, 10, 14] {
        let mut state = conflict_fixture();
        state.mode = AppMode::Confirm;
        state.file_conflict.as_mut().unwrap().conflicts[0].destination =
            format!("root/{}file.txt", "very-long-directory/".repeat(100)).into();
        let text = rendered_dialog(&state, 40, rows);
        assert!(text.contains("Overwrite   Skip   Cancel"), "{rows}: {text}");

        let mut request = state.file_conflict.take().unwrap().request;
        request.operations = vec![FileOperation::Delete {
            source: format!("root/{}file.txt", "very-long-directory/".repeat(100)).into(),
        }];
        state.confirmation = Some(request);
        let text = rendered_dialog(&state, 40, rows);
        assert!(text.contains("Delete   Cancel"), "{rows}: {text}");
    }
}

#[test]
fn confirmation_kinds_distinguish_info_and_warning_dialogs() {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.dialog_kind = DialogKind::Info;
    assert_eq!(
        content(&state, &Theme::emergency_monochrome()).3,
        DialogKind::Info
    );

    state.dialog_kind = DialogKind::Warning;
    assert_eq!(
        content(&state, &Theme::emergency_monochrome()).3,
        DialogKind::Warning
    );

    state.dialog_kind = DialogKind::Error;
    assert_eq!(
        content(&state, &Theme::emergency_monochrome()).3,
        DialogKind::Error
    );
}

#[test]
fn directory_edit_confirmation_lists_colored_changes_and_actions() {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    state.directory_edit.begin(
        PanelId::Left,
        "root".into(),
        &[
            DirectoryEntry {
                path: "root/README.md".into(),
                name: "README.md".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            },
            DirectoryEntry {
                path: "root/compile_commands.json".into(),
                name: "compile_commands.json".into(),
                is_directory: false,
                is_parent: false,
                is_hidden: false,
            },
        ],
        0,
    );
    state.directory_edit.lines[0].text = "README1.md".into();
    state.directory_edit.lines.remove(1);
    state.directory_edit.lines.push(EditLine {
        text: "test.txt".into(),
        original: None,
    });
    state.directory_edit.awaiting_confirmation = true;
    state.dialog_kind = DialogKind::Info;

    let mut theme = Theme::emergency_monochrome();
    theme.mode_insert = Rgb::parse("#A6E3A1").unwrap().into();
    theme.error = Rgb::parse("#F38BA8").unwrap().into();
    theme.dialog_warning = Rgb::parse("#FAB387").unwrap().into();
    theme.directory_edit_create = Rgb::parse("#010203").unwrap().into();
    theme.directory_edit_rename = Rgb::parse("#040506").unwrap().into();
    theme.directory_edit_delete = Rgb::parse("#070809").unwrap().into();
    let (title, lines, height, kind) = content(&state, &theme);

    assert_eq!(title, "Confirmation");
    assert_eq!(
        command_line_text(&state),
        "Confirmation: CREATE test.txt | RENAME README.md → README1.md | DELETE compile_commands.json | [Y]es   [N]o   [C]ancel"
    );
    assert_eq!(kind, DialogKind::Info);
    assert_eq!(height, 7);
    let rendered = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            " CREATE test.txt",
            " RENAME README.md → README1.md",
            " DELETE compile_commands.json",
            "",
            " Yes   No   Cancel",
        ]
    );
    assert_eq!(
        lines[0].spans[1].style.fg,
        Some(theme.directory_edit_create.into())
    );
    assert_eq!(
        lines[1].spans[1].style.fg,
        Some(theme.directory_edit_rename.into())
    );
    assert_eq!(
        lines[2].spans[1].style.fg,
        Some(theme.directory_edit_delete.into())
    );
    let accent = theme.dialog_color(state.dialog_kind);
    for span in [&lines[4].spans[1], &lines[4].spans[3], &lines[4].spans[5]] {
        assert_eq!(span.style.fg, Some(accent.into()));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }
}

#[test]
fn delete_confirmation_names_the_target_and_uses_warning_actions() {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    let target = std::path::PathBuf::from("root/archive");
    state.workspace.panel_mut(PanelId::Left).entries = vec![DirectoryEntry {
        path: target.clone(),
        name: "archive".into(),
        is_directory: true,
        is_parent: false,
        is_hidden: false,
    }];
    let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: "root".into(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    });
    state.confirmation = Some(FileOperationBatchRequest {
        operations: vec![FileOperation::Delete {
            source: target.clone(),
        }],
        source_panel: PanelId::Left,
        protected_directory: "other".into(),
        refresh,
    });
    state.dialog_kind = DialogKind::Warning;
    let mut theme = Theme::emergency_monochrome();
    theme.dialog_warning = Rgb::parse("#FAB387").unwrap().into();

    let (title, lines, height, kind) = content(&state, &theme);
    let rendered = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(title, "Delete");
    assert_eq!(
        command_line_text(&state),
        "Delete: Delete directory permanently? | root/archive | [D]elete   [C]ancel"
    );
    assert_eq!(kind, DialogKind::Warning);
    assert_eq!(height, 6);
    assert_eq!(
        rendered,
        vec![
            " Delete directory permanently?",
            " root/archive",
            "",
            " Delete   Cancel",
        ]
    );
    for span in [&lines[3].spans[1], &lines[3].spans[3]] {
        assert_eq!(span.style.fg, Some(theme.dialog_warning.into()));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    state.workspace.panel_mut(PanelId::Left).entries[0].is_directory = false;
    assert_eq!(
        command_line_text(&state),
        "Delete: Delete file permanently? | root/archive | [D]elete   [C]ancel"
    );
    assert_eq!(
        content(&state, &theme).1[0].spans[0].content,
        " Delete file permanently?"
    );
    state
        .confirmation
        .as_mut()
        .unwrap()
        .operations
        .push(FileOperation::Delete {
            source: "root/second".into(),
        });
    let (_, lines, height, _) = content(&state, &theme);
    assert_eq!(height, 5);
    assert_eq!(
        command_line_text(&state),
        "Delete: Delete 2 selected entries permanently? | [D]elete   [C]ancel"
    );
    assert_eq!(
        lines[0].spans[0].content,
        " Delete 2 selected entries permanently?"
    );
}

fn content(state: &AppState, theme: &Theme) -> (&'static str, Vec<Line<'static>>, u16, DialogKind) {
    let dialog = super::content::build(state, theme, false);
    let lines = dialog
        .body
        .into_iter()
        .chain(dialog.actions)
        .collect::<Vec<_>>();
    let height = u16::try_from(lines.len()).unwrap().saturating_add(2);
    (dialog.title, lines, height, dialog.kind)
}

fn conflict_fixture() -> AppState {
    let mut state = AppState::new(
        ".".into(),
        "test".into(),
        TerminalSize {
            columns: 80,
            rows: 24,
        },
        WorkspaceOptions::default(),
    );
    let metadata = FileMetadata {
        path: "a".into(),
        file_type: "Regular file".into(),
        size: "1 B (1 bytes)".into(),
        created: "Unavailable".into(),
        modified: "Unavailable".into(),
        permissions: "Read/write".into(),
        extension: None,
        hard_links: None,
    };
    let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: ".".into(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    });
    let request = FileOperationBatchRequest {
        operations: ["a", "b"]
            .into_iter()
            .map(|name| FileOperation::Copy {
                source: name.into(),
                destination: format!("to/{name}").into(),
                overwrite: false,
            })
            .collect(),
        source_panel: PanelId::Left,
        protected_directory: "to".into(),
        refresh,
    };
    state.file_conflict = Some(FileConflictState::new(
        request,
        vec![FileConflict {
            source: "a".into(),
            destination: "to/a".into(),
            new_metadata: metadata.clone(),
            existing_metadata: metadata,
        }],
    ));
    state.dialog_kind = DialogKind::Warning;
    state
}

fn rendered_dialog(state: &AppState, columns: u16, rows: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).unwrap();
    terminal
        .draw(|frame| draw(frame, state, &Theme::emergency_monochrome()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(usize::from(columns))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
