use crate::app::{Action, Effect, Notification};
use crate::commands::CommandDispatch;
use crate::config::{ConfigRuntime, LuaRequest};
use crate::theme::Theme;

use super::RuntimeResources;
use super::terminal_writer::TerminalWriter;

mod files;
mod opening;
mod preview;

pub fn execute(
    effect: Effect,
    config: &ConfigRuntime,
    resources: &mut RuntimeResources<'_>,
    writer: &mut TerminalWriter,
    theme: &Theme,
) -> Vec<Action> {
    let RuntimeResources {
        clipboard,
        media,
        finder,
        documents,
        shell,
        operations,
        tree,
    } = resources;

    let result = match effect {
        Effect::SetDynamicFileColorPolicy(policy) => {
            config.set_file_color_policy(policy).map(|()| {
                vec![Action::Notify(Notification::info(format!(
                    "File colors: {}",
                    policy.name()
                )))]
            })
        }
        Effect::LoadHelp => Ok(vec![Action::HelpLoaded(
            crate::help::HelpState::from_keymaps(&config.keymaps()),
        )]),
        Effect::UseTheme(name) => config.use_theme(&name).map(|()| {
            vec![Action::Notify(Notification::info(format!(
                "Theme '{name}' active"
            )))]
        }),
        Effect::UsePalette(name) => config.use_palette(&name).map(|()| {
            vec![Action::Notify(Notification::info(format!(
                "Colorscheme '{name}' active"
            )))]
        }),
        Effect::UseSyntax(name) => config.use_syntax(&name).map(|()| {
            vec![Action::Notify(Notification::info(format!(
                "Syntax '{name}' active"
            )))]
        }),
        Effect::UseDisplayStyle(name) => config.use_style(&name).and_then(|()| {
            Ok(vec![
                Action::ApplyDisplayStyle(config.display_style()?),
                Action::Notify(Notification::info(format!("Style '{name}' active"))),
            ])
        }),
        Effect::RunShellCommand { command, directory } => {
            Ok(shell.start(&command, &directory, config.shell()))
        }
        Effect::CancelShellCommand => Ok(shell.cancel()),
        Effect::CancelFileOperation => {
            operations.cancel();
            Ok(Vec::new())
        }
        Effect::LoadDirectory {
            panel,
            path,
            preferred,
        } => crate::filesystem::load_directory(panel, &path, preferred.as_deref())
            .map(|action| vec![action])
            .or_else(|message| Ok(vec![Action::DirectoryLoadFailed { panel, message }])),
        Effect::OpenExternalFile {
            path,
            application,
            text_editor,
        } => opening::open_file(path, application, text_editor, media, writer, theme),
        Effect::RevealInFileManager { path, is_directory } => {
            crate::file_opening::reveal_in_file_manager(&path, is_directory).map(|()| Vec::new())
        }
        Effect::OpenEditorInDirectory { directory, editor } => {
            opening::open_editor_in_directory(directory, editor, media, writer, theme)
        }
        Effect::CopyPathToClipboard(path) => clipboard.copy_path(&path).map(|()| {
            vec![Action::Notify(Notification::info(
                "Full path copied to clipboard",
            ))]
        }),
        Effect::CopyNameToClipboard(path) => clipboard.copy_name(&path).map(|()| {
            vec![Action::Notify(Notification::info(
                "Name copied to clipboard",
            ))]
        }),
        Effect::PerformFileOperation(request) => files::perform_operation(request, operations),
        Effect::PerformFileOperationBatch(request) => files::perform_batch(request, operations),
        Effect::ApplyDirectoryEdit(request) => files::apply_directory_edit(request, operations),
        Effect::RefreshPanels { requests, context } => Ok(vec![Action::PanelsRefreshed {
            results: crate::filesystem::refresh_panels(requests),
            context,
        }]),
        Effect::LoadPreviewMetadata { generation, path } => {
            preview::load_metadata(generation, path)
        }
        Effect::PreparePreviewMedia {
            generation,
            path,
            panel,
            region,
        } => preview::prepare_media(generation, path, panel, region, media, writer),
        Effect::LoadPreviewText { generation, path } => preview::load_text(generation, path),
        Effect::LoadPreviewDirectory {
            generation,
            path,
            show_hidden,
        } => preview::load_directory(generation, path, show_hidden),
        Effect::LoadPreviewArchive { generation, path } => preview::load_archive(generation, path),
        Effect::StartDocumentPreview(request) => preview::start_document(request, media, documents),
        Effect::CancelDocumentPreview => {
            documents.cancel();
            Ok(Vec::new())
        }
        Effect::PresentDocumentRaster {
            generation,
            request_id,
            path,
            panel,
            region,
            raster,
        } => Ok(vec![preview::document_raster_result(
            generation,
            request_id,
            path,
            media.prepare_raster(&raster, panel, region, writer),
        )]),
        Effect::ClearMediaSurface => media.clear_current(theme, writer).map(|()| Vec::new()),
        Effect::ProbeViewer {
            generation,
            path,
            title,
            size,
            maximum_percent,
        } => preview::probe_viewer(generation, path, title, size, maximum_percent, media),
        Effect::PrepareViewerMedia {
            generation,
            path,
            metadata: _,
            layout,
        } => preview::prepare_viewer(generation, path, layout, media, writer),
        Effect::StartPreviewPlayback {
            generation,
            path,
            size,
            muted,
            panel,
            region,
        } => preview::start_playback(generation, path, size, muted, panel, region, media),
        Effect::StopPreviewPlayback => {
            media.stop();
            Ok(vec![Action::PreviewPlaybackStopped])
        }
        Effect::SetPreviewMuted(muted) => {
            media.set_muted(muted);
            Ok(Vec::new())
        }
        Effect::ExecuteCommandLine(line) => config
            .command_line(&line)
            .and_then(|dispatch| dispatch_command(dispatch, config)),
        Effect::CompleteCommand { line, backwards } => Ok(vec![Action::CommandCompletionsLoaded {
            items: config.command_completions(&line),
            line,
            backwards,
        }]),
        Effect::StartFinderSearch(request) => {
            finder.start(request);
            Ok(Vec::new())
        }
        Effect::CancelFinderSearch => {
            finder.cancel();
            Ok(Vec::new())
        }
        Effect::RevealFinderResult {
            generation,
            panel,
            path,
        } => crate::filesystem::reveal_finder_result(generation, panel, &path)
            .map(|action| vec![action])
            .or_else(|message| {
                Ok(vec![Action::FinderRevealFailed {
                    generation,
                    message,
                }])
            }),
        Effect::LoadTreeChildren {
            generation,
            parent,
            show_hidden,
        } => Ok(vec![
            match crate::filesystem::tree::read_children(&parent, show_hidden) {
                Ok(entries) => Action::TreeChildrenLoaded {
                    generation,
                    parent,
                    entries,
                },
                Err(message) => Action::TreeLoadFailed {
                    generation,
                    message,
                },
            },
        ]),
        Effect::ExpandTreeAll {
            generation,
            root,
            show_hidden,
        } => {
            tree.start(generation, root, show_hidden);
            Ok(Vec::new())
        }
        Effect::RevealTreeEntry {
            generation,
            panel,
            path,
            is_directory,
        } => files::reveal_tree_entry(generation, panel, path, is_directory),
    };

    result.unwrap_or_else(|error| vec![Action::Notify(Notification::error(error))])
}

pub fn dispatch_command(
    dispatch: CommandDispatch,
    config: &ConfigRuntime,
) -> Result<Vec<Action>, String> {
    match dispatch {
        CommandDispatch::Actions(actions) => Ok(actions),
        CommandDispatch::Lua {
            callback,
            invocation,
        } => dispatch_requests(
            config.call_lua(callback, invocation.arguments, invocation.count)?,
            config,
        ),
    }
}

pub fn dispatch_requests(
    requests: Vec<LuaRequest>,
    config: &ConfigRuntime,
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for request in requests {
        match request {
            LuaRequest::Notify(message) => {
                actions.push(Action::Notify(Notification::info(message)))
            }
            LuaRequest::Execute {
                command,
                args,
                count,
            } => actions.extend(dispatch_command(
                config.dispatch(&command, args, count)?,
                config,
            )?),
            LuaRequest::ExecuteLine(line) => {
                actions.extend(dispatch_command(config.command_line(&line)?, config)?)
            }
        }
    }

    Ok(actions)
}
