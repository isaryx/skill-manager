//! Reusable full-screen terminal widgets.
//!
//! Widgets take over the terminal with the alternate screen buffer, draw themselves on every
//! keypress, and restore the previous screen on exit (including panics and `Ctrl-C`).
//!
//! Rendering and key handling are pure functions over widget state, so behavior is unit-tested
//! without a TTY. Only [`MultiSelect::interact`] touches the terminal.

mod multi_select;

pub use multi_select::{MultiSelect, MultiSelectItem};

use std::io;

use dialoguer::console::Term;

const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";
const CLEAR_TO_END_OF_LINE: &str = "\x1b[0K";
const CLEAR_TO_END_OF_SCREEN: &str = "\x1b[0J";

/// Strip characters that would break a widget's drawing contract.
///
/// Widget text comes from profile files and store directory names. A control character — `ESC`
/// above all — would let that text move the cursor, clear the screen, or add rows, desyncing the
/// repaint from the origin [`Screen::draw`] assumes. Today's callers pass IDs already validated
/// against a strict ASCII allowlist, but a widget should not depend on every caller doing that.
///
/// Stripped characters become `U+FFFD` rather than vanishing, so tampering is visible instead of
/// silently changing what a row appears to say.
///
/// Bidi and zero-width characters are deliberately left alone: they measure as zero width, so
/// they cannot break the layout, and removing them would mangle legitimate non-Latin text.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// Owns the alternate screen for as long as it is alive.
///
/// `Drop` restores the terminal, so an early return, an error, or a panic inside a widget loop
/// still leaves the user's scrollback intact.
struct Screen<'a> {
    term: &'a Term,
}

impl<'a> Screen<'a> {
    fn enter(term: &'a Term) -> io::Result<Self> {
        term.write_str(ENTER_ALT_SCREEN)?;
        let screen = Screen { term };
        term.hide_cursor()?;
        term.clear_screen()?;
        Ok(screen)
    }

    /// Repaint the screen from the top. Lines are cleared as they are written, and the tail of
    /// the screen is cleared afterwards, so shrinking output leaves no residue.
    fn draw(&self, lines: &[String]) -> io::Result<()> {
        self.term.move_cursor_to(0, 0)?;
        let mut buf = String::new();
        for (idx, line) in lines.iter().enumerate() {
            buf.push_str(line);
            buf.push_str(CLEAR_TO_END_OF_LINE);
            // No trailing newline on the last line: it would scroll the alternate screen.
            if idx + 1 < lines.len() {
                buf.push_str("\r\n");
            }
        }
        buf.push_str(CLEAR_TO_END_OF_SCREEN);
        self.term.write_str(&buf)?;
        self.term.flush()
    }
}

impl Drop for Screen<'_> {
    fn drop(&mut self) {
        let _ = self.term.show_cursor();
        let _ = self.term.write_str(LEAVE_ALT_SCREEN);
        let _ = self.term.flush();
    }
}
