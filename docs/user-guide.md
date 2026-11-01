# User guide

[Back to the README](../README.md)

This guide describes the shipped controls, preview behavior, file operations, and Lua API. Press `?` in the application to see your active key bindings, including custom mappings.

## Contents

- [Controls](#current-controls)
- [Directory editing](#directory-editing)
- [Preview and image viewer](#rich-preview-and-image-viewer)
- [Archives](#built-in-archives)
- [Search and finder](#search-and-file-finder)
- [Syntax highlighting](#syntax-highlighting)
- [Lua configuration](#lua-configuration)

## Current controls

These defaults are shipped as Lua and can be replaced or removed by user configuration.

| Key | Action |
|---|---|
| `Tab` | Switch the active panel |
| `j`, Down / `k`, Up | Move down / up; an explicit count is supported |
| `gg` / `G` | Select the first / last entry; `Ngg` and `NG` select absolute line `N` |
| `zz` | Center the selected entry when possible |
| `H`, `M`, `L` | Move to the top scroll boundary, middle, or bottom scroll boundary |
| `h` | Open the parent directory |
| `l` | Enter the selected directory; files are ignored |
| Enter | Enter the selected directory, show a supported image internally, or open another file with its configured application |
| `<Space>oe` | Reveal the selected file or open the selected directory in the system file manager |
| `<Space>oc` | Open the active panel directory in the explicitly configured text editor |
| `<Space>s` | Toggle the mark on the selected file or directory |
| `<Space>h` / `:hidden` | Toggle hidden files and directories in both panels |
| `v` | Begin contiguous visual selection at the current entry |
| `Esc` in visual mode | Cancel the visual range while keeping the cursor and manual marks |
| `<Space>p` | Toggle preview in place of the inactive panel |
| `Ctrl-P`, `p` | Start or stop audio/video preview playback while preview is visible |
| `Ctrl-M`, `m` | Mute or unmute preview audio, including before playback starts |
| `Ctrl-D` / `Ctrl-U` | Scroll preview metadata down / up by half a page |
| `]` / `[` | Show the next / previous PDF, DjVu, CBZ, or ebook preview page; counts are supported |
| `Esc`, `q` | Close the modal image viewer while it has focus |
| `F5` | Copy marked entries, or the selected entry when no marks exist, to the other panel |
| `o` / `s` in a floating file-conflict dialog | Overwrite or skip the current copy or move conflict |
| `a` in a floating batch file-conflict dialog | Toggle applying the next choice to all remaining conflicts |
| `c` / `Esc` in a floating file-conflict dialog | Cancel the complete copy or move before any item is changed |
| `F6` | Move marked entries, or the selected entry when no marks exist, to the other panel |
| `Shift+F8` | Request permanent deletion of marked entries or the selected entry; floating dialogs use `d` to delete and `c` or Esc to cancel |
| `/` | Search incrementally within the entries already loaded in the active panel |
| `n` / `N` | Repeat the last confirmed search forward / backward; counts are supported |
| `<Space>ff`, `Alt+F7` | Open the recursive glob finder from the active panel |
| `<Space>t` | Toggle the directory tree rooted at the active panel |
| `<Space>yp` / `<Space>yn` | Copy the current entry's full path / name with extension to the system clipboard |
| `<Space>za` | Prepare ZIP creation from the selection; Enter confirms the output name |
| `<Space>zx` / `<Space>zX` | Extract the current archive into a new folder here / in the other panel |
| `i` | Edit the active directory as a text buffer |
| `:` | Enter command mode |
| `Up` / `Down` in command mode | Recall older / newer submitted commands; Down after the newest restores the draft |
| `Esc` | Cancel a draft/search session or leave command mode |
| `:layout <framed\|borderless\|compact>` | Change panel and footer layout together |
| `:border <rounded\|plain\|double\|thick\|simple>` | Change every visible frame type |
| `:separator <rounded\|plain\|double\|thick\|simple\|dashed\|off>` | Change divider lines |
| `:colorscheme <name>` / `:syntax <name\|off>` | Change UI or syntax colors independently |
| `:style <name>` / `:theme <name>` | Apply a named display style or UI theme with its syntax |
| `:icons <nerd_font\|unicode\|none>` | Change icon display for this session |
| `:name-colors <on\|off>` | Toggle icon-colored file names for this session |
| `:file-color-policy <nearest\|original>` | Compare matched and original devicons colors in the generated theme |
| `:q`, `q` | Exit |

The panels always divide the available workspace 50/50 and retain independent directories, selections, marks, and scroll positions. Both start at the supplied path. A `●` marker identifies every manually marked file or directory; the synthetic `..` entry cannot be marked. `v` enters `VISUAL` mode and highlights the contiguous range between its fixed anchor and the moving cursor. Normal Vim motions and counts extend or shrink that range. Esc drops only the visual range and keeps the cursor and manual marks. Copy, move, and delete use the union of the visual range and manual marks, or the current entry when neither exists. Starting a copy or move returns to `NORMAL`; accepted deletion does the same, while cancelling its confirmation returns to `VISUAL`. A batch attempts every independent item, refreshes the panels once, and clears manual marks after completion. Directories sort before files, a synthetic `..` entry is shown away from a filesystem root, and the default relative line numbering reserves four columns. The built-in Mocha theme follows Catppuccin's `LineNr` and `CursorLine` treatment: ordinary numbers use Surface1, the current number uses a Green accent, and the selected row uses the Mocha cursor-line blend. Terminal resize stops playback, clears the old media region, redraws the layout, and prepares the selected preview for the new viewport.

Open panel directories are watched for external changes even when the terminal is not focused. The watcher uses non-recursive native notifications where available, combines bursts over 100 ms, and reads updated listings off the UI thread. Both panels retain their selection, marks, and scroll position when possible; a panel whose directory was removed moves to the nearest existing parent. Selected-file changes also refresh its preview. A ten-second background reconciliation catches missed directory-entry events, including on filesystems that do not reliably emit notifications. Refresh application waits until directory editing, file operations, and modal views close. `:refresh` remains available for an immediate manual reload; automatic monitoring of nested directories outside the open panels is not performed.

Panel and footer layouts can differ in a named Lua display style. `framed` draws complete frames; `borderless` uses title bands and no outer panel frames; `compact` draws a partial frame between panels and above the footer. `:layout` changes both layouts for the current session. Define or derive a style in `init.lua` to choose the two layouts separately. Preview continues to replace the inactive panel and uses its configured preview colors.

Mouse buttons and the wheel do not control the application. Button and wheel reports are captured and discarded so terminals such as WezTerm cannot translate the wheel into navigation keys. Pointer-motion reporting is not enabled. Mouse reporting is released when leaving the application or launching an external program.

Framed titles retain three horizontal border cells and one space on each side when truncated. A title that does not fit ends with `...`; truncation respects terminal-cell widths and whole grapheme clusters. Titles that fit are shown in full.

File presentation is controlled by two independent style properties:

```lua
wolf.style.icons = "nerd_font" -- nerd_font, unicode, none
wolf.style.color_file_names = true
```

`nerd_font` uses original glyphs and source RGB colors from pinned dark and light tables of `nvim-web-devicons`, with exact filename matches taking precedence over compound and ordinary extensions. The generated `terminal` theme selects the table from terminal appearance; fixed dark themes use the dark table. `unicode` uses standard symbols (`🗀`, `🗁`, `🗎`, `↩`). `none` removes file icons and padding, appends `/` to directory labels, and uses ASCII tree markers. Icon selection does not control title edges or Powerline separators; the named style controls those decorations. `:icons none` can be combined with `:border double` and `:separator double`. `simple` uses ASCII frame and divider glyphs. Filenames and preview content are not transliterated. The old `ascii` icon value is accepted as an alias for `none`.

To combine Far colors and syntax with the classic display style in user `init.lua`:

```lua
wolf.theme.use("far")
wolf.style.use("classic")
```

Set `icon_variant = "light"`, `"dark"`, or `"auto"` in a theme to choose which pinned devicons table supplies source colors. The default is `auto`: it uses the UI background when known, then terminal appearance. A direct `wolf.theme.icon_variant` assignment overrides the selected theme.

Set `wolf.style.frames.border` and `wolf.style.separator` in Lua to change the active style. Session `:border` and `:separator` take precedence until another style is selected.

The generated `terminal` theme uses source devicons RGB colors by default for icons and colored file names. Use `:file-color-policy nearest` to try 16-color palette matching, then `:file-color-policy original` to restore the default. This session setting resets on a theme or colorscheme switch and is unavailable for Far and other fixed themes. With no reported 16-color terminal palette, `nearest` uses the existing terminal foreground fallback.

The display style controls whether file names are colored; `:name-colors on|off` changes that setting for the session. Catppuccin keeps the original devicons RGB colors. Far maps each known file's devicons color to the nearest readable color in its fixed 16-color palette; the result is shared by its icon, when enabled, and its name. Generated `terminal` uses source RGB colors from the light or dark devicons table. If `nearest` is selected without a complete reported 16-color palette, it uses the ordinary terminal color fallback. Names remain uncolored when their style setting is off. Directory and hidden-entry roles take precedence. The same rules apply in panels, directory editing, finder, tree, and directory/archive preview. Hidden filesystem attributes are carried into finder results without filesystem access during drawing. Four upstream Lua tables and their MIT license are vendored under `Vendor/NvimWebDevicons`. Nearest-palette matching remains an internal theme policy rather than a public Lua property.

The directory tree opens in a centered float with the same bounds as the recursive finder. Its root and immediate children are visible initially; nested directories are lazy and collapsed. Use `j`/`k`, counts, `gg`, `G`, `zz`, `H`, `M`, and `L` for navigation, Tab to toggle a directory, `h`/`l` to collapse or expand, `zR`/`zM` to expand or collapse the complete tree, and Enter to reveal the selected directory or file in the panel that opened the tree. Esc or `q` closes it.

`zR` runs in a background worker with a 10,000-entry budget and a depth limit of 128. Closing or collapsing the tree cancels pending expansion between filesystem calls. Exceeding a limit reports an error and retains the existing tree. Individual directory listings also have a 10,000-entry limit. Directory links and Windows reparse points remain leaves.

## Directory editing

Press `i` in `NORMAL` mode to turn only the active panel into an editable directory buffer. Existing entries remain tied to their original filesystem paths while their visible names are edited. Changing a line renames that entry, clearing or removing its line deletes it, and adding a non-empty line creates an entry. A new line ending in `/` creates a directory; any other new line creates an empty file. Unicode input and cursor movement with Left, Right, Up, Down, Home, and End are supported, while Enter and Backspace split and join lines like a text editor.

Esc compares the edit buffer with its opening snapshot. If the filesystem diff is empty, editing closes immediately. Otherwise a three-way confirmation lists every `CREATE`, `RENAME`, and `DELETE`, leaves one blank row, and then shows a separate `Yes   No   Cancel` action row. In the floating dialog, fixed `y`/`n`/`c` shortcuts respectively apply the complete diff, discard the buffer, or return to `INSERT`; Esc also cancels the question. Create uses the theme's insert green, delete uses its error red, and rename uses its warning orange. The bold `Y`/`N`/`C` shortcuts all follow the dialog frame accent. Confirmations use floating dialogs by default and keep the visible application mode at `NORMAL`; only the float contains the interaction. They can be returned to the command line with `wolf.option.set("ui.dialogs", "command_line")`, which displays `COMMAND` and applies the command-mode colour to a mode-following footer frame. Invalid names, duplicates, destination conflicts, protected directories, and unsafe cyclic renames are rejected with a visible error; an unsuccessful save keeps the edit buffer open.

Three layouts are available for panels and footer: `framed`, `borderless`, and `compact`. The default `modern` style uses framed panels and footer with rounded borders, including when the `terminal` theme is selected. `:layout <name>` changes both areas for the session; `wolf.style.panels.layout` and `wolf.style.footer.layout` can set them independently in Lua. In every layout, the status line shows the current mode plus status information, and the command line receives input and notifications. `status.mode_position` places the mode segment on the left or right.

Themes may disable the framed footer's dynamic perimeter with `framed_footer_border_follows_mode = false`; `framed_footer_border` then supplies its static colour. The shipped theme keeps mode-following enabled. Existing custom themes that omit these fields retain the same default and use `status_border` as the static fallback. Generated `terminal` uses terminal ANSI colors 6 for SEARCH, 2 for INSERT, 3 for COMMAND, 4 for NORMAL, and 5 for VISUAL. With a reported 16-color palette, its compact mode badge uses the mode color as its background and selects contrasting text from the same palette. The fixed built-in palettes keep their own mode colors.

The directory-edit confirmation has independent operation-label colors in the Lua theme: `directory_edit_create` (default `#A6E3A1`), `directory_edit_rename` (`#FAB387`), and `directory_edit_delete` (`#F38BA8`). They affect only `CREATE`, `RENAME`, and `DELETE` in the change list; the dialog frame, title, and action shortcuts keep the dialog-kind accent. Older themes without these fields retain their previous insert-mode, warning, and error color fallbacks respectively.

Command-line confirmation prompts replace previous notifications and show a visible cursor after the question. Long prompts scroll horizontally to keep the answer choices and cursor visible; floating dialogs do not claim the command-line cursor.

## Rich preview and image viewer

In framed panel mode, the preview border uses `borderless_preview_title_background` (Mocha Mauve), and its title uses `active_title` (Mocha Green). The finder preview keeps its own frame colours.

Panel and finder previews leave one blank cell on each side of the metadata text and its horizontal separators. Long paths wrap within the value column, with continuation lines aligned below the path rather than the field label. Metadata paging includes these continuation lines, so the complete path remains accessible in a short viewport.

The style's `separator` field controls content divider lines independently of frame borders: `plain` (`─` / `│`), `rounded`, `double`, `thick`, `simple` (`-` / `|`), `dashed` (`┄` / `┆`), or `off`. `off` leaves divider cells blank without changing layout dimensions. `:separator <type>` overrides it for the session. In the compact layout, the line between panels uses the panel border style and the line above the footer uses the footer border style; neither depends on `separator`, including `off`. Border types accept only `rounded`, `plain`, `double`, `thick`, and `simple`; `:border <type>` applies one type to all visible frames. The legacy Lua value `thin` remains an alias for `plain` separators.

Panel previews allocate 70% of the available content height to the preview and 30% to metadata by default, matching the finder. Text previews, ebook text, and directory/archive listings leave one blank cell on each side in both locations.

`<Space>p` replaces the inactive file panel without destroying its directory, selection, or viewport. The preview keeps a fixed upper content region and a separately scrollable lower metadata region. Files, directories, and symbolic links show filesystem metadata in a stable order: full path, type, size, creation and modification times, platform permissions, extension, and hard-link count where available. A directory preview lists its immediate child directories first and then its files, without recursive traversal; an empty one is marked `Empty directory`. ZIP containers (`.zip`, `.jar`, `.war`, `.ear`, and `.apk`), `.7z`, and uncompressed `.tar` archives show their indexed internal paths without extracting data; an empty archive is marked `Empty archive`. Images and videos additionally show source dimensions. Audio-only previews show type, duration, playback, and mute without a meaningless resolution row. Plain text, source code, Markdown, JSON, configuration files, logs, and other UTF-8 or BOM-marked UTF-16 text are rendered directly in the upper region. Text reads, directory listings, and archive indexes are bounded, and binary files are rejected rather than decoded lossily. Unsupported files remain useful metadata-only previews.

PDF and DjVu pages are decoded and rasterized in pure Rust; CBZ images are ordered naturally and decoded without extracting the archive to disk. EPUB, MOBI/PRC, AZW3, KFX, and FB2 have plain-text reflow decoders rendered by Ratatui; format coverage depends on the decoder and the document, and unsupported variants report an unavailable preview. `]`, `[`, and numeric counts navigate pages, while the metadata area shows the current and total count plus available title, author, and page dimensions. Ebook pagination uses wrapped terminal-cell widths and preserves grapheme clusters; a viewport too narrow for one complete grapheme reports that it must be widened. Document work runs in a runtime-owned worker, caches the current and adjacent pages, ignores stale generations, and re-renders the current page after resize. Cancellation discards stale results and stops subsequent work; an active synchronous decoder call must finish before another request or shutdown completes. Fox Media Term receives only a ready RGBA raster and remains responsible for terminal-protocol output.

Document input is capped at 256 MiB. CBZ/EPUB archives are checked before decoding against 50,000 entries and 128 MiB of declared uncompressed data. CBZ source images have separate decode limits (16 million pixels, 16,384 per edge, and the image decoder's 64 MiB allocation budget), in addition to the configured output render-pixel limit. These are separate bounds, not a single process-wide memory cap.

This is a fast file-manager preview rather than a full document reader: ebook HTML/CSS, complex fixed layouts, annotations, interactive forms, and DRM are not reproduced. Encrypted PDFs and DRM-protected ebooks report a clear unavailable state. Image viewer and audio/video controls are unchanged.

Images and video thumbnails use the same automatically selected Fox Media Term backend as standalone media output. Audio and video never autoplay. `Ctrl-P`/`p` starts or stops the selected playable preview; every new start begins at the beginning. `Ctrl-M`/`m` changes mute both during playback and before the next start without stopping playback. Audio-only files render a Ratatui amplitude visualizer from pre-mute PCM snapshots, so it continues moving while muted and never owns an external terminal graphics surface. Selection changes, panel switches, finder or viewer entry, file mutation, resize, preview close, and exit stop playback; metadata scrolling does not. Preview playback deliberately has no pause/resume, seek, volume, playlist, or background-playback behavior.

The audio visualizer draws a centered, mirrored amplitude ribbon with fine Braille contours, subdued inner traces, and a continuous gradient between the theme's `preview_visualizer_low` and `preview_visualizer_high` colors. Neighboring PCM levels are smoothed and the ribbon tapers at the edges; silence settles to a quiet center line. Its padded, bounded size leaves breathing room in large previews while adapting to small viewports.

Media decoder selection is extension-gated before content is opened: image extensions follow stb_image's supported formats, with WebP decoded by FFmpeg. Audio and video use an explicit list of media extensions, followed by FFmpeg content probing. FFmpeg's full demuxer extension list is not used because it also includes raw samples, subtitles, and extensions shared with ordinary documents (such as Visual Studio `.sln` files). Stream inspection classifies a regular video stream as video and an audio-only file as audio; attached cover art does not turn MP3, FLAC, or M4A into video. Other text files use the Ratatui text preview; unsupported binary formats have no media preview.

Pressing Enter on a supported image opens a centered modal viewer. Its bounds default to 80% of terminal width and height, preserve aspect ratio, account for terminal cell pixel geometry and text fallback geometry, and never upscale a smaller source image. Esc or `q` closes the modal and restores rich preview when enabled. `l` is directory navigation only and never opens a file. Enter resolves external files in this order: an exact extension association, the configured text editor when the bounded content probe recognizes text, and finally the operating system default. The text editor runs in the foreground with Commander Wolf's terminal temporarily suspended, so terminal editors such as Neovim remain interactive; after it exits, the TUI and preview are restored. If the editor cannot be launched or exits unsuccessfully, the operating system default is tried automatically. Setting `viewer.use_internal` to `false` disables only the modal image viewer; image preview remains available and Enter follows the external-opening hierarchy.

`:reveal` and `<Space>oe` use the system file manager without changing either Commander Wolf panel. Directories open directly. Windows Explorer and macOS Finder select files; Linux and other Unix desktops open the containing directory through `xdg-open`, because there is no portable selection API. Failure to locate or launch the file manager is reported in the command line without closing Commander Wolf.

`:editor` and `<Space>oc` start the explicitly configured text editor in the active panel root and pass `.` as its directory target. No operating-system editor fallback is used when `set_text_editor` is absent. `:editor <program> [args...]` supplies a one-shot override, for example `:editor nvim` or `:editor code --new-window`. Terminal editors run in the foreground while Commander Wolf suspends and restores its TUI; GUI editor launchers may return immediately.

`:copypath` and `<Leader>yp` copy the current entry's absolute path; `:copyname` and `<Leader>yn` copy its name with all extensions. Spaces and Unicode are preserved, without surrounding quotes or an added newline. Both work for files and directories in normal and visual modes and leave marks, visual selection, and media playback unchanged. They copy only the current entry, regardless of other marked entries; for `..`, the actual parent directory's path or name is used. The stable Lua command IDs are `file.copy_path` and `file.copy_name`.

Clipboard access uses the operating system, independently of the terminal: Windows and macOS native clipboards, and Linux Wayland data-control with X11 fallback. A desktop clipboard must be available; headless sessions report an error. On Linux, Commander Wolf retains clipboard ownership while running; keeping text after exit depends on a desktop clipboard manager.

Available file commands are:

```text
:cd <path>              change only the active panel directory
:refresh                reload both panels
:reveal                 reveal the selected entry in the system file manager
:editor [program ...]    open the active panel root in a text editor
:copy                   copy the selection to the other panel
:copypath               copy the current entry's full path to the system clipboard
:copyname               copy the current entry's name with extension to the system clipboard
:archive [name.zip]     create a ZIP, or prepare an editable output name
:extract [new-folder]   extract the current archive into a new folder
:extractother           extract into a new folder in the other panel
:move                   move the selection to the other panel
:delete                 request permanent recursive deletion
:rename [new-name]      rename directly, or ask for a new name
:mkdir <path>           create one directory
:touch <path>           safely create one empty file
```

Paths may be quoted and may contain spaces or Unicode. `touch`, rename, and mkdir fail when the destination already exists. Copy and move never overwrite silently: a conflict offers overwrite, skip, or cancellation before the batch starts. Floating file-conflict dialogs own fixed, non-configurable `a`/`o`/`s`/`c` shortcuts for the MVP; `Esc` also cancels. The title identifies `Copy conflict` or `Move conflict`. The `a` toggle is shown only for batch operations and applies the next overwrite or skip choice to every remaining conflict. Command-line dialogs continue to use the shipped `confirm` keymaps. Confirmed overwrite preserves the old destination under a temporary backup while publishing the replacement and then removes that backup. Directory copy does not follow directory symlinks; unsupported symlink copies fail explicitly. Cross-device move copies first and removes the source only after a complete copy. A partial result is reported precisely and both panels are refreshed whenever the filesystem may have changed.

`:rename new-name` renames the selected file or directory immediately. Bare `:rename` opens an editable name prompt using `ui.dialogs`; `<Leader>r` (Space then r by default) always opens the themed `Rename` float via `file.rename_prompt`. The old name is prefilled, with the cursor at the end. Left/Right, Home/End, Backspace/Delete, Ctrl+A (select all), and Ctrl+U (clear) edit the field; Enter confirms and Esc or Ctrl+C cancels. Confirming an unchanged name closes the prompt without a filesystem operation. Empty names and path separators show an inline validation error. While the prompt is open, keys belong to it, not the file panels. Floating rename keeps `NORMAL`; command-line rename uses `COMMAND`. Both submit through the existing typed rename implementation, preserving spaces and Unicode without interpreting the name as a command.

### Built-in archives

`:archive "backup.zip"` creates a compressed ZIP from the union of manually marked entries and the visual range, or the current entry when neither exists. Files, hidden entries, nested directories and empty directories are included; each selected root keeps its own name in the archive. Bare `:archive` or `<Leader>za` opens the command line with a suggested ZIP name; edit it if needed and press Enter. A destination without an extension gets `.zip`. Paths are relative to the active panel, and their parent directory must already exist. Creation also works in visual mode.

`:extract` or `<Leader>zx` extracts the current ZIP, uncompressed TAR or 7z archive into a sibling folder named after the archive without its final extension. ZIP variants recognized by preview, such as CBZ and JAR, are also accepted. `:extract "new folder"` supplies another destination; `:extractother` or `<Leader>zX` puts the new folder in the other panel. Extraction uses only the current entry, regardless of marks. External archivers are not required.

Both operations run in the existing background worker with byte progress and Esc/Ctrl+C cancellation. Completed output is published only after the entire operation succeeds. Existing destinations are never overwritten or merged; choose another name when one already exists. Failure or cancellation removes temporary output, with any cleanup failure reporting the remaining path. Completion refreshes both panels and selects the result when it is directly inside one of them. ZIP creation clears marks after completion, as copy/move batches do.

This initial implementation creates new ZIPs and does not update existing archives. Extraction rejects absolute paths, parent traversal, conflicting file/directory paths, links, special files and Windows device/stream names. Source symlinks and special files are not archived. Limits are 100,000 entries, 64 GiB of declared extracted data, 4,096 bytes per entry path and 128 path components. Unix file permission bits are retained, excluding special bits; ownership, timestamps and directory permissions are not restored. Password-protected archives, RAR and compressed TAR formats such as `.tar.gz` are not supported; decoder failures leave no published result. The stable Lua IDs are `archive.create`, `archive.extract` and `archive.extract_to_other`; default keymaps remain replaceable through `wolf.keymap`.

## Search and file finder

`:hidden` toggles hidden entries in both panels, the directory tree, and directory previews. Hidden entries include dot-prefixed names and the Windows Hidden or System attributes. Their icons and names use the selected UI palette's `hidden` color. The status line shows `Hidden: on` or `Hidden: off` in every footer style. All entries remain visible by default; set `workspace.show_hidden` to `false` to start with hidden entries concealed. Toggling preserves the selected path when it remains visible and clears marks on entries that become hidden. The finder retains its separate `finder.show_hidden` search setting.

`/` searches only the current panel's in-memory directory entries. Matching is case-sensitive substring search, updates while Unicode text is entered, highlights every occurrence, and wraps around the list. Esc before Enter cancels the draft and restores its original selection. Enter confirms the query and keeps `SEARCH` mode visible; `n`, `N`, `5n`, and `3N` then repeat it without recursively accessing the filesystem. Esc after confirmation returns to `NORMAL`, removes the search highlights, disables `n` / `N`, and leaves the current selection in place.

The finder opens in `INSERT` mode and starts no work until Enter submits a valid glob expression. Results stream from a cancellable background worker and become a sorted Vim-navigable list when the scan completes or is cancelled. In result `NORMAL` mode, `j`, `k`, counts, `gg`, `G`, `zz`, `H`, `M`, and `L` behave like panel navigation; `i` edits the pattern, Enter reveals the selected file or directory in its containing directory, and `q` or Esc closes the finder.

Supported glob syntax includes:

```text
*.cpp
Source/*.cpp
Source/**/Command*.cpp
File?.[ch]
Test[abc].{cpp,h}
*.cpp;*.h;!vendor/**
!Build/**;!*.tmp
Source/
*\Tests\
```

Patterns without a separator match basenames at any depth; patterns with `/` or `\` match normalized relative paths. `**` crosses directory separators, braces may be nested, a leading `!` excludes matches, and a trailing slash means directories only. Matching is deterministically case-sensitive on every platform. Hidden entries are skipped by default, directory symlinks are never traversed, and a configurable result limit prevents unbounded result lists.

## Syntax highlighting

Text previews in both the panel and finder use syntect's bundled grammars, detected from the filename, extension, or first-line shebang. Code, JSON, YAML, Markdown, and other recognized formats receive highlighting; unknown formats remain plain text. No external editor or syntax installation is required. Syntect uses the pure-Rust `regex-fancy` backend.

The pipeline is `SyntaxHighlighter → semantic SyntaxSpan categories → Lua palette → Ratatui Span`. `SyntectHighlighter` maps parser scopes to application-owned categories without exposing syntect types to preview rendering. A future backend can implement the same interface. Tokens are computed when text is loaded and retained with that preview, not reparsed during drawing. Highlighting stops after 1,024 lines, 128 KiB, or a line longer than 4,096 bytes; the remaining preview stays readable as plain text. Parser failures also retain the text and show a diagnostic when space permits.

Each theme selects syntax colors by name or defines them inline. The generated `terminal` theme uses terminal ANSI colors; each Catppuccin theme selects its matching Latte, Frappé, Macchiato, or Mocha syntax palette from `runtime/lua/commander_wolf/syntax/`. The `dracula`, `gruvbox_dark`, `gruvbox_light`, `nord`, `one_dark`, `solarized_dark`, `solarized_light`, and `tokyonight_night` themes select their matching authored syntax palettes. `:syntax <name>` replaces only the syntax colors for this session, and `:syntax off` displays cached text without token styling. To define a separate syntax preset in user `init.lua`:

```lua
wolf.syntax.define("my_syntax", {
  text = "#CDD6F4",
  comment = "#9399B2",
  keyword = "#CBA6F7",
  string = "#A6E3A1",
  number = "#FAB387",
  ["function"] = "#89B4FA",
})
wolf.syntax.use("my_syntax")
```

`text` is required for a standalone syntax palette; a palette with `extends = "existing_name"` may specify only changed categories. Other supported keys are `escape`, `boolean`, `type`, `variable`, `parameter`, `property`, `constant`, `operator`, `punctuation`, `tag`, `attribute`, `preprocessor`, `heading`, `link`, `code`, `emphasis`, `strong`, `added`, `removed`, and `invalid`. Colors accept `#RRGGBB`, `default`, `ansi0` through `ansi15`, and terminal palette names. Invalid values produce Lua errors. Palette changes recolor cached tokens without reparsing. Comments and emphasis are italic, headings and strong text are bold, and links are underlined; preview backgrounds remain controlled by the UI palette.

## Lua configuration

Visual ranges use the UI palette's `visual_selection_background` and `visual_selection_text`. These override icon, filename, and search-match colors inside the range for contrast; the ordinary cursor row uses `selected` and the optional `selected_text`. Older flat themes fall back to `selected` and `text` when these fields are absent. With a reported 16-color terminal palette, generated `terminal` uses terminal color 4 for the cursor row and selects a text color from that palette by contrast, preferring neutral colors; without it, the previous background-based selection remains. Its directory names, active borders, titles, and current line numbers also use color 4. Visual ranges use a neutral surface; search matches are derived from terminal yellow (3) and the current match from cyan (6). Its active and inactive borderless title backgrounds use the corresponding border colors, independently of row selection. `borderless_inactive_title_text` may optionally set a separate text color for the inactive title; if omitted, it uses `borderless_title_text`.

### Dialog text and shortcuts

Command-line confirmations use the same questions, details, and action labels as floating dialogs, flattened onto one line with ` | ` between sections. Shortcut letters are bracketed without individual color accents: `[D]elete   [C]ancel`, `[Y]es   [N]o   [C]ancel`, and `[O]verwrite   [S]kip   [C]ancel`. Batch conflicts show the same checkbox and `Apply to [a]ll remaining conflicts`; single-file conflicts omit it. The shipped `confirm` keymaps accept these letters in either case; legacy `y`/`n`, Space (toggle all), and Esc remain available, and user Lua may still override the mappings. Rename prompts show `New name` and `[Enter] Rename   [Esc] Cancel   [Ctrl+A] Select all`, matching the float's controls.

### File operation progress

Copy, move and delete always display a modal progress float, independently of `ui.dialogs`. This also covers directory edits that include deletion. The float uses the current theme and border style, shows the current file and overall progress, and blocks all commands/navigation until the worker stops. Escape, `c` (or `C`), and Ctrl+C request cancellation; the UI stays locked while safe cleanup finishes. Existing overwrite/skip/cancel conflict dialogs and delete confirmations are resolved before work starts.

File work runs off the UI thread. An initial cancellable scan estimates the total work. Copy uses byte progress; empty files and directory entries receive a minimal work unit. Delete advances by filesystem entries rather than fictitious byte transfer. A same-filesystem move is an atomic rename, so its current-file bar advances from zero to complete; cross-filesystem moves report copying and source removal separately. Totals represent processed work, not a guarantee that every item succeeded. Failures and partial results are reported through the existing command-line messages and affected panels are refreshed.

Cancellation is cooperative: copies check at native copy progress callbacks on Windows and between 256 KiB blocks elsewhere; recursive operations check between entries. Already completed work is not rolled back. Interrupted copies clean up staging and leave the original source and old destination intact when cleanup succeeds; cleanup failures include the recovery path in the error. Publication/backup restoration finishes as a transaction before cancellation takes effect. Cross-filesystem moves keep the complete destination and any remaining source if cancellation interrupts source cleanup. Deletion is permanent, including entries removed before cancellation. A blocked operating-system filesystem call must return before cancellation can finish.

### External shell commands

Use `:!<command>` (with or without a space after `!`) to execute a command in the active panel's directory. The shell receives the original command text, including quoting, pipelines and redirections. Interactive submissions are stored in command history, and `!` appears in command completion.

Output from stdout and stderr streams into a themed float above the bottom edge, capped at half the terminal height. The UI remains responsive: `Up`/`Down`, `PageUp`/`PageDown`, and `Home`/`End` scroll the output. `Ctrl+C` terminates the command's process tree (Windows) or process group (Unix); this is termination, not interactive terminal input. Once finished, Enter or Esc closes the float, and `:` closes it and starts a new command. Both panels refresh on dismissal. Launch failures and nonzero exit codes remain visible in the float. Native preview images are hidden underneath it until dismissal.

The default shell is PowerShell (`pwsh`, falling back to `powershell`) on Windows and `$SHELL` (falling back to `/bin/sh`) on Linux/macOS. Automatic PowerShell execution uses no profile, UTF-8 output and plain-text formatting. Override the executable and arguments in `init.lua`; the command text is appended as a single final argument:

```lua
wolf.option.set("shell", { program = "pwsh", args = { "-NoLogo", "-NoProfile", "-NonInteractive", "-Command" } })
-- Other examples:
-- wolf.option.set("shell", { program = "cmd.exe", args = { "/D", "/S", "/C" } })
-- wolf.option.set("shell", { program = "/bin/bash", args = { "-c" } })
-- wolf.option.set("shell", nil) -- Restore automatic selection.
```

This is a captured-output runner, not a terminal emulator: stdin is closed, so interactive prompts and full-screen applications are not supported. Output is decoded as UTF-8 with replacement for invalid bytes; custom shells/programs should emit UTF-8. ANSI color, cursor and terminal-control sequences are stripped. The first 1 MiB or 10,000 lines are retained, with a truncation notice; excess output is drained without growing memory or blocking the child. Separate stdout/stderr pipes preserve each stream's order, not exact ordering between streams. Windows commands belong to an owned Job Object; Unix commands belong to a process group. Remaining members are terminated when the parent exits, on cancellation, or on application shutdown, and output readers are joined. Unix descendants that deliberately leave the process group are outside this ownership policy; inherited pipes from such escaped processes can still delay shutdown. Shell commands run with the application's permissions and have their normal filesystem side effects; there is no additional confirmation or undo.

### Command completion

In command mode, `Tab` opens a floating list of public command names with descriptions and inserts the first match. At an empty `:` it lists all commands; otherwise it matches the typed name prefix. `Tab` and `Shift+Tab` cycle forward/backward with wraparound. The list uses the current float background, border style, title and selection colors, stays above the command line, and occupies at most half the terminal height. Long lists scroll with a scrollbar; narrow terminals clip descriptions. Native preview images are temporarily hidden while the list is open and restored on dismissal.

Enter submits the selected command. Typing, Backspace, or history navigation closes the list and continues editing. Esc first closes it and restores the original input; another Esc exits command mode. No match leaves the input unchanged. Fixed arguments for `:layout`, `:border`, `:separator`, `:icons`, and `:name-colors` complete by prefix. `:colorscheme`, `:syntax`, `:style`, and `:theme` complete from the live component registries, including user definitions. Paths and free-form arguments do not complete. The default bindings `command.complete_next` and `command.complete_previous` can be replaced or removed using `wolf.keymap`.

The `:file-color-policy` arguments `nearest` and `original` also complete by prefix.

### Command history

In command mode, `Up` and `Down` browse the last 500 submitted commands. Recalled text can be edited before Enter; editing does not change the stored entry. Down after the newest command restores the draft that was present before browsing. Esc discards the draft without recording it. Empty commands and consecutive exact duplicates are omitted; invalid submitted commands are retained. Commands longer than 4,096 UTF-8 bytes still execute but are not stored. Long input scrolls to keep the tail and cursor visible.

History loads automatically on startup and saves on normal exit, independently of `session.remember_directories`. It lives in `command_history.dat` under `wolf.fn.stdpath("state")` (on Windows, `%LOCALAPPDATA%/commander-wolf`). The private versioned data file is not executed or loaded as Lua. Missing history is normal; corrupt/unreadable history produces a diagnostic and starts empty, while save failures are reported after leaving the terminal UI. Completed writes atomically replace the old file. Abrupt termination does not save the current session; with multiple instances, the last successful save wins. `--clean` neither reads nor writes history. This file contains the entered command text, including its arguments.

The default arrow bindings are shipped in Lua as `command.history_previous` and `command.history_next` and can be replaced or removed through the usual `wolf.keymap` API. History only records interactive command-line submissions, not normal-mode shortcuts or commands executed by Lua.

### Remember panel directories

Session memory is disabled by default: both panels start at the launch directory (or the explicit command-line path). To enable it, add this to user `init.lua`:

```lua
wolf.option.set("session.remember_directories", true)
```

On normal exit, the current left and right directories are saved and restored on the next launch. An explicit path, including `.`, takes precedence and opens both panels there; that run still saves its final directories if enabled. `--clean` retains the disabled shipped default. An unavailable saved directory falls back independently to the startup path with a diagnostic. Missing state is normal; corrupt/unreadable state also falls back with a diagnostic. Save failures are reported after leaving the terminal UI. Abrupt termination does not save the current session. With several instances, the last successful save wins.

Only the two paths are stored, not selections, marks, unsaved edits, or UI settings. The private versioned `session.dat` file lives under `wolf.fn.stdpath("state")`: `%LOCALAPPDATA%/commander-wolf` on Windows, or `$XDG_STATE_HOME/commander-wolf` (default `$HOME/.local/state/commander-wolf`) on Linux/macOS. Writes replace the previous state only after a complete temporary file has been written. Disabling the option neither reads nor modifies an existing session file; re-enabling it can restore that saved state. The state file is data, not executable Lua or an additional configuration format.

### User configuration

Commander Wolf embeds vendored Lua 5.5 through `mlua`. Missing user configuration is normal. The optional entry point is:

- Windows: `%LOCALAPPDATA%/commander-wolf/init.lua`
- Linux and macOS: `$XDG_CONFIG_HOME/commander-wolf/init.lua`, falling back to `$HOME/.config/commander-wolf/init.lua`

Commander Wolf never creates this file or copies its defaults into the user directory. `--clean` ignores `init.lua` while retaining all shipped defaults. A broken user configuration is discarded transactionally; the application starts with shipped defaults and reports the Lua path and traceback.

The same host API table is available as the global `wolf` and through `require`:

```lua
local wolf = require("commander_wolf")
assert(rawequal(wolf, _G.wolf))

wolf.g.mapleader = " "
wolf.keymap.del("normal", "q")
wolf.keymap.set("normal", "<Leader>q", "app.quit", { description = "Quit Commander Wolf" })

wolf.command.create("hello_wolf", function(context)
  wolf.api.notify("Hello from Lua; count=" .. tostring(context.count or "none"))
end, { description = "Show a greeting" })

wolf.option.set("workspace.line_numbers", "relative") -- off, absolute, relative
wolf.option.set("workspace.scroll_offset", 10)
wolf.style.icons = "nerd_font" -- nerd_font, unicode, none
wolf.style.color_file_names = false
wolf.option.set("workspace.show_hidden", true)
wolf.option.set("session.remember_directories", false)
wolf.option.set("finder.show_hidden", false)
wolf.option.set("finder.scroll_offset", 10)
wolf.option.set("finder.max_results", 10000)
wolf.option.set("preview.media_percent", 70) -- 20..80
wolf.option.set("preview.documents.enabled", true)
wolf.option.set("preview.documents.cache_pages", 3) -- 3..32
wolf.option.set("preview.documents.max_render_pixels", 8000000) -- 65536..64000000
wolf.option.set("viewer.max_percent", 80) -- 20..100
wolf.option.set("viewer.use_internal", true)
wolf.option.set("status.mode_position", "left") -- left, right
wolf.option.set("ui.dialogs", "float") -- float, command_line
wolf.theme.use("catppuccin_mocha")

wolf.file_association.set("md", "markdown-viewer")
wolf.file_association.set_text_editor("nvim")
wolf.file_association.set({ "jpg", "jpeg" }, {
  program = "image-editor",
  args = { "--reuse-window" },
})
wolf.file_association.del("jpeg") -- del also accepts an extension list
-- wolf.file_association.clear_text_editor() removes the text fallback
-- wolf.file_association.clear() removes every custom association

wolf.keymap.del("normal", "<F5>")
wolf.api.command("file.rename", { "new name.txt" })
```

The optional fourth argument to `wolf.keymap.set` accepts a string `description`. All shipped mappings provide one. Rust code can read descriptions through `Keymaps::bindings(context)`; replacing or deleting a mapping also replaces or removes its description.

Press `?` in normal mode to open the help float. It shows Commander Wolf 0.1.0 and every active key binding across contexts, including mappings from user `init.lua`. Descriptions come from the keymap definitions; mappings without one show `(no description)`. Use Up/Down, PageUp/PageDown, Home/End, or `j`/`k` to scroll, and Esc or `q` to close it.

The custom command is then available as `:hello_wolf`. Keymap right-hand sides may be stable command IDs or Lua functions. Available path helpers are `wolf.fn.stdpath("config"|"data"|"state"|"cache"|"runtime")`. File-association extensions are case-insensitive, may include compound suffixes such as `tar.gz`, and accept either one string or an array. A string application names one executable; the table form accepts `program` and an optional `args` array, with the opened path appended as a separate final argument. `set_text_editor` accepts the same application forms without requiring an extension list and is also the configured source for `:editor`; the latter runs from the active panel root with `.` instead of appending a file path. The API also provides keymap `set`/`del`/`clear`, command `alias`/`create`/`del`/`execute`, file-association `set`/`del`/`clear` plus text-editor fallback controls, theme and component `define`/`use`, and typed `wolf.api.notify`/`wolf.api.command` requests.

The shipped Lua sources under `runtime/lua/commander_wolf/` register themes, syntax presets, styles, aliases, and mappings. The startup combination is theme `catppuccin_mocha` with style `modern`. Built-in Catppuccin themes are `catppuccin_latte`, `catppuccin_frappe`, `catppuccin_macchiato`, and `catppuccin_mocha`; each uses its official palette for UI and syntax colors. Latte uses light file icons automatically. The `dracula` theme uses fixed Dracula colors, with purple directories and active borders. The `gruvbox_dark` and `gruvbox_light` themes use Gruvbox's medium contrast palettes, with green directories and active borders; the light variant uses light file icons automatically. The `nord` theme uses fixed Nord colors, with Frost blue directories and active borders. The `one_dark` theme uses fixed Atom One Dark colors, with blue directories and active borders. The `solarized_dark` and `solarized_light` themes use the original Solarized palette with opposite background and text tones; both have blue directories and active borders, and the light variant uses light file icons automatically. The `tokyonight_night` theme uses the Night variant's fixed colors, with blue directories and active borders. Style `nvim` has borderless panels/footer, rounded floating frames, and ordinary file names; `modern` has rounded frames and ordinary file names; `classic` has double panel frames, a borderless footer, colored file names, and no icons or Powerline decoration. Theme `far` supplies fixed RGB UI and syntax colors from a classic 16-color palette; it does not inherit terminal palette colors. Its panels use light-cyan text and frames on blue, black cursor text on cyan, white directories, and cyan hidden entries, following [Far Manager default color roles](https://github.com/FarGroup/FarManager/blob/master/far/palette.cpp) and [file highlighting](https://github.com/FarGroup/FarManager/blob/master/far/hilight.cpp). The footer uses a black background, including its frame, dividers, and command line. Shared floating surfaces retain the blue viewer background. Themes and styles can be combined freely.

Far shows the cursor-row highlight only in the active panel. Its path title is bright yellow, the inactive title is bright cyan, and the preview title is bright magenta, all on the blue panel background. Each panel keeps its cursor position when focus changes. The optional Lua theme fields `framed_active_title_background` and `framed_active_title_text` customize the active file-panel title in framed layouts; when omitted, the existing panel background and active title color are used.

Far uses fixed mode colors from its 16-color palette: green for INSERT, bright cyan for SEARCH, bright magenta for VISUAL, and bright yellow for COMMAND (the palette has no orange). NORMAL retains white text in the framed footer and black text on cyan in borderless and compact footers. The optional `compact_mode_normal_background` theme field overrides the NORMAL badge background; other modes use `compact_mode_background` when set, or their mode color otherwise.

Far's ordinary floating dialogs use black text on light gray; warnings and errors use white text on dark red. Shortcut hints remain bold and use cyan on gray or bright yellow on red. Rename prompts and file-operation progress use the same dialog surfaces. Directory trees, file finders, and image viewers retain their blue floating background. Optional theme fields `dialog_info_background`, `dialog_info_text`, `dialog_warning_background`, `dialog_warning_text`, `dialog_error_background`, and `dialog_error_text` control these surfaces. `dialog_info_shortcut`, `dialog_warning_shortcut`, and `dialog_error_shortcut` override shortcut colors independently of borders and body text. When omitted, dialogs retain `float_background`, `text`, and their existing severity accents. Command-line dialogs keep the footer colors.

The generated `terminal` theme has no Lua theme file. Its UI follows terminal appearance and its syntax uses terminal ANSI colors. While the TUI is active, terminal appearance detection asynchronously queries OSC 11 for the background and OSC 4 for the 16 ANSI colors at startup, after external editors, on focus return, and periodically. Light/dark appearance is inferred from the reported background; missing reports use a usable fallback. Terminal replies are separated from keyboard input before keymaps, including replies that arrive after the original request. `wolf.option.set("terminal.appearance", "light"|"dark"|"auto")` controls only the generated UI colors. A user definition named `terminal` takes precedence over the generated theme. `:theme <name>` changes the UI theme and its syntax for this session while preserving the selected style; it does not write configuration.

User `init.lua` can call `require("themes.mine")` to load one file at `config/commander-wolf/themes/mine.lua`. That file may define themes, syntax presets, and styles. For example, derive a style and apply it with a built-in theme:

```lua
wolf.style.define("my_modern", {
  extends = "modern",
  panels = { border = "double" },
})
wolf.theme.use("catppuccin_mocha")
wolf.style.use("my_modern")
```

To keep the `terminal` theme and color file names in user `init.lua`:

```lua
wolf.theme.use("terminal")
wolf.style.color_file_names = true
```

The public appearance API has three independent objects: `wolf.theme`, `wolf.syntax`, and `wolf.style`. Each supports `define(name, config)`, `use(name)`, and direct reading or assignment of its effective properties. A theme defines UI colors and selects syntax by name or inline definition. `extends` inherits from a theme, syntax preset, or style of the same kind:

```lua
wolf.theme.define("my_theme", {
  extends = "catppuccin_mocha",
  base = "#101014",
  syntax = { extends = "catppuccin_mocha", keyword = "#cba6f7" },
})
wolf.theme.use("my_theme")
wolf.style.use("modern")
```

`wolf.theme.use("far")` selects Far UI colors and its initial syntax. `wolf.syntax.use("catppuccin_mocha")` can then replace only the syntax; `wolf.style.use("classic")` changes only presentation. Direct assignments such as `wolf.theme.base = "#101014"`, `wolf.syntax.keyword = "#cba6f7"`, and `wolf.style.color_file_names = true` override individual effective properties. Reading them returns the effective values. A theme `use` clears UI and syntax assignments and the separate syntax selection while keeping the selected style and its assignments. Syntax or style `use` clears only assignments for that object. Unknown presets, properties, and invalid values raise Lua errors. `:colorscheme` switches only UI colors; `:theme` selects UI colors and syntax together. A combined theme-and-style preset is not part of this API yet.

Theme colors accept `#RRGGBB`, `default` for terminal foreground/background, `ansi0` through `ansi15`, and the case-sensitive palette names `black`, `red`, `green`, `yellow`, `blue`, `purple`, `cyan`, `white` and their `bright` forms. `magenta` and `brightMagenta` alias `purple` and `brightPurple`. The theme owns semantic UI colors and its syntax reference; the display style owns layouts, frame and divider types, decorative glyphs, icon mode, and file-name color toggle. Original known-file RGB colors still come from pinned `nvim-web-devicons` data for RGB themes. Every shipped mapping and alias can be removed or replaced through Lua.
