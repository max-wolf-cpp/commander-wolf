use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::AppState;
use crate::dialogs::{DialogKind, FileConflict, FileConflictState};
use crate::directory_edit::DirectoryEditChange;
use crate::file_operations::{FileOperation, FileOperationBatchRequest};
use crate::theme::Theme;

pub(super) struct DialogContent {
    pub title: &'static str,
    pub body: Vec<Line<'static>>,
    pub actions: Vec<Line<'static>>,
    pub kind: DialogKind,
}

pub(super) fn build(state: &AppState, theme: &Theme, bracket_keys: bool) -> DialogContent {
    let colors = theme.dialog_colors(state.dialog_kind);
    let shortcut = theme.dialog_shortcut_color(state.dialog_kind, colors.accent);
    let key = |text: &'static str| {
        if bracket_keys {
            Span::raw(format!("[{text}]"))
        } else {
            dialog_key(text, shortcut)
        }
    };

    if let Some(conflict_state) = &state.file_conflict
        && let Some(conflict) = conflict_state.current()
    {
        return conflict_content(state, conflict_state, conflict, &key);
    }

    if state.directory_edit.awaiting_confirmation {
        return directory_edit_content(state, theme, &key);
    }

    if let Some(request) = state
        .confirmation
        .as_ref()
        .filter(|request| request.is_delete_confirmation())
    {
        return delete_content(state, request, &key);
    }

    DialogContent {
        title: "Confirmation",
        body: vec![Line::from(" Confirm operation?"), Line::from("")],
        actions: vec![Line::from(vec![
            Span::raw(" "),
            key("Y"),
            Span::raw("es   "),
            key("N"),
            Span::raw("o   "),
            key("C"),
            Span::raw("ancel"),
        ])],
        kind: state.dialog_kind,
    }
}

fn conflict_content(
    state: &AppState,
    conflict_state: &FileConflictState,
    conflict: &FileConflict,
    key: &impl Fn(&'static str) -> Span<'static>,
) -> DialogContent {
    let body = vec![
        Line::from(" File already exists"),
        Line::from(Span::styled(
            format!(" {}", conflict.destination.display()),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        metadata_line("New", &conflict.new_metadata),
        metadata_line("Existing", &conflict.existing_metadata),
        Line::from(""),
    ];
    let mut actions = Vec::new();
    if conflict_state.is_batch() {
        actions.push(Line::from(vec![
            Span::raw(format!(
                " [{}] Apply to ",
                if conflict_state.apply_to_all {
                    "x"
                } else {
                    " "
                }
            )),
            key("a"),
            Span::raw("ll remaining conflicts"),
        ]));
        actions.push(Line::from(""));
    }
    actions.push(Line::from(vec![
        Span::raw(" "),
        key("O"),
        Span::raw("verwrite   "),
        key("S"),
        Span::raw("kip   "),
        key("C"),
        Span::raw("ancel"),
    ]));

    DialogContent {
        title: match conflict_state.request.kind() {
            Some(crate::file_operations::FileOperationKind::Move) => "Move conflict",
            _ => "Copy conflict",
        },
        body,
        actions,
        kind: state.dialog_kind,
    }
}

fn directory_edit_content(
    state: &AppState,
    theme: &Theme,
    key: &impl Fn(&'static str) -> Span<'static>,
) -> DialogContent {
    let mut body = state
        .directory_edit
        .changes()
        .into_iter()
        .map(|change| directory_change_line(change, theme, state.dialog_kind))
        .collect::<Vec<_>>();
    body.push(Line::from(""));
    let actions = vec![Line::from(vec![
        Span::raw(" "),
        key("Y"),
        Span::raw("es   "),
        key("N"),
        Span::raw("o   "),
        key("C"),
        Span::raw("ancel"),
    ])];

    DialogContent {
        title: "Confirmation",
        body,
        actions,
        kind: state.dialog_kind,
    }
}

fn delete_content(
    state: &AppState,
    request: &FileOperationBatchRequest,
    key: &impl Fn(&'static str) -> Span<'static>,
) -> DialogContent {
    let mut body = match request.operations.as_slice() {
        [FileOperation::Delete { source }] => vec![
            Line::from(format!(
                " Delete {} permanently?",
                delete_target_noun(state, request.source_panel, source)
            )),
            Line::from(Span::styled(
                format!(" {}", source.display()),
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ],
        operations => vec![Line::from(format!(
            " Delete {} selected entries permanently?",
            operations.len()
        ))],
    };
    body.push(Line::from(""));
    let actions = vec![Line::from(vec![
        Span::raw(" "),
        key("D"),
        Span::raw("elete   "),
        key("C"),
        Span::raw("ancel"),
    ])];

    DialogContent {
        title: "Delete",
        body,
        actions,
        kind: state.dialog_kind,
    }
}

fn delete_target_noun(
    state: &AppState,
    panel: crate::workspace::PanelId,
    source: &std::path::Path,
) -> &'static str {
    state
        .workspace
        .panel(panel)
        .entries
        .iter()
        .find(|entry| entry.path == source)
        .map_or("entry", |entry| {
            if entry.is_directory {
                "directory"
            } else {
                "file"
            }
        })
}

fn directory_change_line(
    change: DirectoryEditChange,
    theme: &Theme,
    kind: DialogKind,
) -> Line<'static> {
    let (operation, color, description) = match change {
        DirectoryEditChange::Create { name } => ("CREATE", theme.directory_edit_create, name),
        DirectoryEditChange::Rename {
            source,
            destination,
        } => (
            "RENAME",
            theme.directory_edit_rename,
            format!("{source} → {destination}"),
        ),
        DirectoryEditChange::Delete { name } => ("DELETE", theme.directory_edit_delete, name),
    };

    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            operation,
            Style::default()
                .fg(theme.dialog_text_color(kind).unwrap_or(color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {description}")),
    ])
}

fn dialog_key(text: &'static str, accent: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        text,
        Style::default().fg(accent).add_modifier(Modifier::BOLD),
    )
}

fn metadata_line(label: &'static str, metadata: &crate::preview::FileMetadata) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {label:<10}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{:<24} {}", metadata.size, metadata.modified)),
    ])
}
