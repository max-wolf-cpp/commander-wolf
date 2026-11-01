#[cfg(windows)]
use std::io::IsTerminal;
use std::io::{self, Write};
use std::num::NonZeroU16;

use crossterm::Command;
use crossterm::event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange};
use fox_media_term::{Buffer, Region};
use ratatui::backend::{Backend, IntoCrossterm};
use ratatui::buffer::CellDiffOption;
use ratatui::layout::Rect;
use ratatui::style::Color;
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    ENABLE_EXTENDED_FLAGS, ENABLE_MOUSE_INPUT, ENABLE_QUICK_EDIT_MODE, GetConsoleMode,
    GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
};

use crate::app::AppState;
use crate::terminal::TerminalSize;
use crate::theme::Theme;

pub struct TerminalWriter {
    terminal: ratatui::DefaultTerminal,
    output: io::Stdout,
    focus_reporting: bool,
    mouse_scrolling_saved: bool,
    #[cfg(windows)]
    native_mouse_mode: Option<u32>,
    resumed: bool,
    wide_supplementary_icons: bool,
    damaged_media: Option<Rect>,
}

impl TerminalWriter {
    pub fn initialize() -> Self {
        let mut writer = Self {
            terminal: ratatui::init(),
            output: io::stdout(),
            focus_reporting: false,
            mouse_scrolling_saved: false,
            #[cfg(windows)]
            native_mouse_mode: None,
            resumed: false,
            wide_supplementary_icons: false,
            damaged_media: None,
        };

        writer.wide_supplementary_icons = probe_supplementary_icon_width(&mut writer.output);
        writer.enable_focus_reporting();
        writer.ignore_mouse_input();
        writer
    }

    pub fn size(&self) -> Result<TerminalSize, String> {
        self.terminal
            .size()
            .map(|size| TerminalSize {
                columns: size.width,
                rows: size.height,
            })
            .map_err(|error| format!("cannot read terminal size: {error}"))
    }

    pub fn draw(&mut self, state: &AppState, theme: &Theme) -> Result<(), String> {
        let wide_icons = self.wide_supplementary_icons
            && state.workspace.icon_mode == crate::icons::IconMode::NerdFont;

        let damaged = self.damaged_media;
        let mut repaint = Vec::new();
        self.terminal
            .draw(|frame| {
                crate::ui::draw(frame, state, theme);
                if wide_icons {
                    align_supplementary_icons(frame.buffer_mut());
                }
                if let Some(area) = damaged {
                    repaint = media_repaint_cells(frame.buffer_mut(), area);
                }
            })
            .map_err(|error| format!("cannot draw Ratatui UI: {error}"))?;

        if !repaint.is_empty() {
            // Native cleanup changed the terminal outside Ratatui's previous buffer.
            // Repaint this region once, including cells that compare equal to that buffer.
            self.terminal
                .backend_mut()
                .draw(repaint.iter().map(|(x, y, cell)| (*x, *y, cell)))
                .and_then(|()| Backend::flush(self.terminal.backend_mut()))
                .map_err(|error| format!("cannot repaint cleared media region: {error}"))?;
        }
        self.damaged_media = None;

        Ok(())
    }

    pub fn write_query(&mut self, request: &[u8]) -> Result<(), String> {
        self.output
            .write_all(request)
            .and_then(|()| self.output.flush())
            .map_err(|error| format!("cannot write terminal query: {error}"))
    }

    pub fn write_media(&mut self, payload: &Buffer) -> Result<(), String> {
        let _is_ansi_utf8 = payload.uses_ansi_utf8();
        self.output
            .write_all(payload.bytes())
            .and_then(|()| self.output.flush())
            .map_err(|error| format!("cannot write media payload: {error}"))
    }

    pub fn clear_media(
        &mut self,
        payload: &Buffer,
        region: Region,
        background: Color,
    ) -> Result<(), String> {
        let area = Rect::new(
            region.column.max(0) as u16,
            region.row.max(0) as u16,
            region.columns.max(0) as u16,
            region.rows.max(0) as u16,
        );
        let bytes = media_cleanup_bytes(payload.bytes(), area, background);
        self.output
            .write_all(&bytes)
            .and_then(|()| self.output.flush())
            .map_err(|error| format!("cannot clear media surface: {error}"))?;
        self.damaged_media = Some(self.damaged_media.map_or(area, |old| old.union(area)));

        Ok(())
    }

    pub fn suspend(&mut self) {
        self.disable_focus_reporting();
        self.restore_mouse_input();
        ratatui::restore();
    }

    pub fn resume(&mut self) {
        self.terminal = ratatui::init();
        self.output = io::stdout();
        self.wide_supplementary_icons = probe_supplementary_icon_width(&mut self.output);
        self.enable_focus_reporting();
        self.ignore_mouse_input();
        self.damaged_media = None;
        self.resumed = true;
    }

    pub fn take_resumed(&mut self) -> bool {
        std::mem::take(&mut self.resumed)
    }

    fn enable_focus_reporting(&mut self) {
        self.focus_reporting = crossterm::execute!(self.output, EnableFocusChange).is_ok();
    }

    fn disable_focus_reporting(&mut self) {
        if self.focus_reporting {
            let _ = crossterm::execute!(self.output, DisableFocusChange);
            self.focus_reporting = false;
        }
    }

    fn ignore_mouse_input(&mut self) {
        #[cfg(windows)]
        {
            if self.native_mouse_mode.is_none() {
                self.native_mouse_mode = capture_console_mouse_input().ok();
            }
        }

        let commands = ignored_mouse_input(!self.mouse_scrolling_saved);
        if self
            .output
            .write_all(&commands)
            .and_then(|()| self.output.flush())
            .is_ok()
        {
            self.mouse_scrolling_saved = true;
        }
    }

    fn restore_mouse_input(&mut self) {
        if self.mouse_scrolling_saved {
            let _ = self.output.write_all(&restored_mouse_input());
            let _ = self.output.flush();
            self.mouse_scrolling_saved = false;
        }

        #[cfg(windows)]
        if let Some(original) = self.native_mouse_mode
            && restore_console_mouse_input(original).is_ok()
        {
            self.native_mouse_mode = None;
        }
    }
}

impl Drop for TerminalWriter {
    fn drop(&mut self) {
        self.disable_focus_reporting();
        self.restore_mouse_input();
    }
}

fn ignored_mouse_input(save_scrolling: bool) -> Vec<u8> {
    let mut commands = String::new();
    if save_scrolling {
        commands.push_str("\x1b[?1007s");
    }
    commands.push_str("\x1b[?1007l\x1b[?9l\x1b[?1001l\x1b[?1005l\x1b[?1016l");
    let _ = DisableMouseCapture.write_ansi(&mut commands);
    // Some hosts turn the wheel into arrow keys whenever mouse reporting is off,
    // regardless of mode 1007. Capture buttons and the wheel, then discard their
    // events in the input loop. Do not request pointer-motion or drag reports.
    commands.push_str("\x1b[?1000h\x1b[?1006h");
    commands.into_bytes()
}

fn restored_mouse_input() -> Vec<u8> {
    let mut commands = String::new();
    let _ = DisableMouseCapture.write_ansi(&mut commands);
    commands.push_str("\x1b[?1007r");
    commands.into_bytes()
}

#[cfg(windows)]
fn capture_console_mouse_input() -> io::Result<u32> {
    // The standard input handle is borrowed from the process, not owned here.
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut original = 0;
    if unsafe { GetConsoleMode(input, &mut original) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let captured =
        (original | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS) & !ENABLE_QUICK_EDIT_MODE;
    if unsafe { SetConsoleMode(input, captured) } == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(original)
}

#[cfg(windows)]
fn restore_console_mouse_input(original: u32) -> io::Result<()> {
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut current = 0;
    if unsafe { GetConsoleMode(input, &mut current) } == 0 {
        return Err(io::Error::last_os_error());
    }

    if unsafe { SetConsoleMode(input, restored_console_mouse_mode(current, original)) } == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(windows)]
fn restored_console_mouse_mode(current: u32, original: u32) -> u32 {
    // Raw-mode restoration may already have run in Ratatui's panic hook.
    // Restore only the mouse flags, preserving the current keyboard input mode.
    let mouse_flags = ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS | ENABLE_QUICK_EDIT_MODE;
    (current & !mouse_flags) | (original & mouse_flags)
}

fn media_cleanup_bytes(native: &[u8], area: Rect, background: Color) -> Vec<u8> {
    let mut bytes = native.to_vec();
    if matches!(background, Color::Rgb(..)) {
        return bytes;
    }

    // The native cleanup API accepts RGB only. Keep its protocol-specific erasure,
    // then apply the exact terminal background without guessing palette/default RGB.
    let mut paint = String::from("\x1b7");
    let _ =
        crossterm::style::SetBackgroundColor(background.into_crossterm()).write_ansi(&mut paint);
    let blank = " ".repeat(usize::from(area.width));
    for row in area.y..area.bottom() {
        use std::fmt::Write as _;
        let _ = write!(paint, "\x1b[{};{}H{blank}", row + 1, area.x + 1);
    }
    paint.push_str("\x1b[0m\x1b8");
    bytes.extend_from_slice(paint.as_bytes());

    bytes
}

fn media_repaint_cells(
    buffer: &ratatui::buffer::Buffer,
    area: Rect,
) -> Vec<(u16, u16, ratatui::buffer::Cell)> {
    let area = area.intersection(buffer.area);
    let mut previous = ratatui::buffer::Buffer::empty(area);
    for cell in &mut previous.content {
        cell.set_diff_option(CellDiffOption::Skip);
    }
    let mut current = ratatui::buffer::Buffer::empty(area);
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            current[(x, y)] = buffer[(x, y)].clone();
        }
    }

    previous
        .diff_iter(&current)
        .map(|(x, y, cell)| (x, y, cell.clone()))
        .collect()
}

#[cfg(windows)]
fn probe_supplementary_icon_width(output: &mut io::Stdout) -> bool {
    use crossterm::cursor::{Hide, MoveTo};
    use crossterm::style::Print;
    use crossterm::terminal::{Clear, ClearType};

    // Only a native console can report the cursor without a terminal reply query.
    if !output.is_terminal() {
        return false;
    }

    let icon = crate::icons::resolve(
        crate::icons::IconMode::NerdFont,
        "..",
        crate::icons::EntryIconKind::Parent,
    );
    let wide = crossterm::execute!(output, Hide, MoveTo(0, 0), Print(icon.glyph))
        .and_then(|()| crossterm::cursor::position())
        .is_ok_and(|position| position == (2, 0));

    let restored = crossterm::execute!(output, MoveTo(0, 0), Clear(ClearType::CurrentLine)).is_ok();
    wide && restored
}

#[cfg(not(windows))]
fn probe_supplementary_icon_width(_output: &mut io::Stdout) -> bool {
    false
}

fn align_supplementary_icons(buffer: &mut ratatui::buffer::Buffer) {
    let columns = usize::from(buffer.area.width);
    if columns == 0 {
        return;
    }

    // These consoles consume the icon's reserved spacer as its second cell.
    // Tell Ratatui to skip that cell and position the next text explicitly.
    for row in buffer.content.chunks_mut(columns) {
        for column in 0..row.len().saturating_sub(1) {
            if row[column].diff_option != CellDiffOption::None
                || row[column + 1].diff_option != CellDiffOption::None
                || row[column + 1].symbol() != " "
            {
                continue;
            }

            let mut characters = row[column].symbol().chars();
            let Some(character) = characters.next() else {
                continue;
            };
            if characters.next().is_none()
                && matches!(character, '\u{f0000}'..='\u{ffffd}' | '\u{100000}'..='\u{10fffd}')
            {
                row[column]
                    .set_diff_option(CellDiffOption::ForcedWidth(NonZeroU16::new(2).unwrap()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::{Buffer, CellDiffOption};
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use super::{
        align_supplementary_icons, ignored_mouse_input, media_cleanup_bytes, media_repaint_cells,
        restored_mouse_input,
    };

    #[test]
    fn mouse_buttons_and_wheel_are_captured_without_motion_or_scroll_key_translation() {
        for save_scrolling in [false, true] {
            let commands = String::from_utf8(ignored_mouse_input(save_scrolling)).unwrap();
            for mode in [9, 1001, 1002, 1003, 1005, 1007, 1015, 1016] {
                assert!(commands.contains(&format!("\x1b[?{mode}l")));
                assert!(!commands.contains(&format!("\x1b[?{mode}h")));
            }
            assert_eq!(commands.starts_with("\x1b[?1007s"), save_scrolling);
            assert!(commands.ends_with("\x1b[?1000h\x1b[?1006h"));
        }

        let restored = String::from_utf8(restored_mouse_input()).unwrap();
        for mode in [1000, 1002, 1003, 1006, 1015] {
            assert!(restored.contains(&format!("\x1b[?{mode}l")));
            assert!(!restored.contains(&format!("\x1b[?{mode}h")));
        }
        assert!(restored.ends_with("\x1b[?1007r"));
    }

    #[cfg(windows)]
    #[test]
    fn restoring_mouse_flags_preserves_keyboard_and_raw_mode_changes() {
        use windows_sys::Win32::System::Console::{
            ENABLE_ECHO_INPUT, ENABLE_EXTENDED_FLAGS, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT,
            ENABLE_PROCESSED_INPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_VIRTUAL_TERMINAL_INPUT,
            ENABLE_WINDOW_INPUT,
        };

        let cooked_keyboard = ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
        let original = cooked_keyboard | ENABLE_QUICK_EDIT_MODE | ENABLE_EXTENDED_FLAGS;
        for keyboard in [0, cooked_keyboard, ENABLE_VIRTUAL_TERMINAL_INPUT] {
            let captured =
                keyboard | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
            assert_eq!(
                super::restored_console_mouse_mode(captured, original),
                keyboard | ENABLE_WINDOW_INPUT | ENABLE_QUICK_EDIT_MODE | ENABLE_EXTENDED_FLAGS,
            );
        }
    }

    #[test]
    fn cleanup_preserves_protocol_erasure_and_restores_terminal_backgrounds() {
        let area = Rect::new(4, 2, 3, 2);
        for protocol in [
            fox_media_term::Protocol::Kitty,
            fox_media_term::Protocol::Sixel,
        ] {
            for (background, command) in [
                (Color::Reset, "\x1b[49m"),
                (Color::Indexed(4), "\x1b[48;5;4m"),
                (Color::Indexed(231), "\x1b[48;5;231m"),
                (Color::White, "\x1b[48;5;15m"),
                (Color::Black, "\x1b[48;5;0m"),
            ] {
                let native = fox_media_term::prepare_cleanup(
                    protocol,
                    fox_media_term::Region {
                        column: 4,
                        row: 2,
                        columns: 3,
                        rows: 2,
                    },
                    [0, 0, 0],
                )
                .unwrap();
                let bytes = media_cleanup_bytes(native.bytes(), area, background);
                assert!(bytes.starts_with(native.bytes()));
                let paint = std::str::from_utf8(&bytes[native.bytes().len()..]).unwrap();
                let command = if crossterm::style::Colored::ansi_color_disabled_memoized() {
                    "\x1b[m"
                } else {
                    command
                };
                assert_eq!(
                    paint,
                    format!("\x1b7{command}\x1b[3;5H   \x1b[4;5H   \x1b[0m\x1b8")
                );
                let deletes_kitty = bytes.windows(7).any(|part| part == b"a=d,d=A");
                assert_eq!(deletes_kitty, protocol == fox_media_term::Protocol::Kitty);
            }
        }
        let native = b"native RGB cleanup";
        assert_eq!(
            media_cleanup_bytes(native, area, Color::Rgb(1, 2, 3)),
            native
        );
    }

    #[test]
    fn repaint_repairs_external_writes_without_touching_new_media_or_other_cells() {
        use ratatui::Terminal;
        use ratatui::backend::{Backend, TestBackend};

        let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
        let render = |frame: &mut ratatui::Frame<'_>| {
            frame
                .buffer_mut()
                .set_string(0, 0, "same UI ", Color::White);
            frame.buffer_mut()[(0, 1)].set_diff_option(CellDiffOption::Skip);
        };
        terminal.draw(render).unwrap();
        let expected = terminal.backend().buffer().clone();
        let mut corrupted = expected.clone();
        for cell in &mut corrupted.content {
            cell.set_symbol("#").set_bg(Color::Black);
        }
        terminal
            .backend_mut()
            .draw(
                corrupted
                    .content
                    .iter()
                    .enumerate()
                    .map(|(index, cell)| ((index % 8) as u16, (index / 8) as u16, cell)),
            )
            .unwrap();

        let frame = terminal.draw(render).unwrap();
        let repaint = media_repaint_cells(frame.buffer, Rect::new(0, 0, 8, 2));
        terminal
            .backend_mut()
            .draw(repaint.iter().map(|(x, y, cell)| (*x, *y, cell)))
            .unwrap();
        let actual = terminal.backend().buffer();
        for x in 0..8 {
            assert_eq!(actual[(x, 0)], expected[(x, 0)]);
        }
        assert_eq!(actual[(0, 1)].symbol(), "#");

        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_string(0, 0, "󰁯 text", Color::White);
        align_supplementary_icons(&mut buffer);
        let repaint = media_repaint_cells(&buffer, Rect::new(0, 0, 3, 1));
        assert_eq!(
            repaint.iter().map(|(x, _, _)| *x).collect::<Vec<_>>(),
            [0, 2]
        );

        buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
        buffer.set_string(0, 0, "界 text", Color::White);
        let repaint = media_repaint_cells(&buffer, Rect::new(0, 0, 4, 1));
        assert_eq!(
            repaint.iter().map(|(x, _, _)| *x).collect::<Vec<_>>(),
            [0, 2, 3]
        );
    }

    #[test]
    fn first_frame_positions_text_and_border_after_a_double_cell_icon() {
        let previous = Buffer::empty(Rect::new(0, 0, 6, 1));
        let mut next = previous.clone();
        next.set_string(0, 0, "│󰁯 x │", Color::White);

        align_supplementary_icons(&mut next);

        let updates = previous.diff_iter(&next).collect::<Vec<_>>();
        assert_eq!(
            updates
                .iter()
                .map(|(x, _, cell)| (*x, cell.symbol()))
                .collect::<Vec<_>>(),
            [(0, "│"), (1, "󰁯"), (3, "x"), (4, " "), (5, "│")]
        );

        // Model a backend that elides cursor moves between adjacent updates.
        let mut logical_previous = None;
        let mut cursor = 0;
        let mut text_position = None;
        let mut border_position = None;
        for (x, _, cell) in updates {
            if logical_previous != Some(x.saturating_sub(1)) {
                cursor = x;
            }
            if cell.symbol() == "x" {
                text_position = Some(cursor);
            }
            if x == 5 {
                border_position = Some(cursor);
            }
            cursor += if cell.symbol() == "󰁯" { 2 } else { 1 };
            logical_previous = Some(x);
        }
        assert_eq!(text_position, Some(3));
        assert_eq!(border_position, Some(5));
        assert_eq!(next.diff_iter(&next).count(), 0);
    }

    #[test]
    fn icon_width_does_not_consume_text_borders_or_skipped_media_cells() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 4));
        buffer.set_string(0, 0, "󰁯│", Color::White);
        buffer.set_string(0, 1, "󰁯 ", Color::White);
        buffer[(1, 1)].set_diff_option(CellDiffOption::Skip);
        buffer.set_string(0, 2, "󰁯 ", Color::White);
        buffer[(0, 2)].set_diff_option(CellDiffOption::Skip);
        buffer[(1, 3)].set_symbol("󰁯");

        let original = buffer.clone();
        align_supplementary_icons(&mut buffer);
        assert_eq!(buffer, original);
    }
}
