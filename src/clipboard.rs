//! System-clipboard integration for yank.
//!
//! Every yank funnels through [`crate::editor::behavior::YankBehaviorState`],
//! which mirrors the yanked text into the OS clipboard via this module.
//!
//! ## Why the handle is long-lived
//!
//! On X11 (and Wayland) the clipboard is *ownership*, not storage: the process
//! that copied must stay alive to serve the bytes when something later pastes.
//! `arboard` serves those requests from a background context tied to the
//! `Clipboard` instance's lifetime. Creating a fresh `Clipboard` per yank and
//! dropping it immediately therefore loses the copy the moment the function
//! returns — unless a clipboard manager happens to be running to take over.
//!
//! So we keep a single `Clipboard` alive for the lifetime of the thread that
//! does the editing (the input loop, which is single-threaded — hence a
//! `thread_local!` rather than a `Mutex`, sidestepping `arboard::Clipboard`'s
//! non-`Send` platform backends).
//!
//! When no native clipboard can be opened (headless, SSH, bare tmux) we fall
//! back to an OSC 52 escape sequence. OSC 52 is write-only but works wherever
//! the terminal forwards it.

/// Mirror `text` into the system clipboard, best-effort.
///
/// Failures are swallowed: an editor with no reachable clipboard should keep
/// working off its internal yank register, not error out.
pub(crate) fn set_system_clipboard(text: &str) {
    // The crate's own test suite compiles with this module active; touching the
    // real clipboard or emitting OSC 52 to stdout there would be a side effect
    // on the developer's machine and would corrupt captured test output. The
    // escape encoding itself is covered by `osc52_sequence`'s unit tests.
    #[cfg(not(test))]
    {
        if set_native(text) {
            return;
        }
        emit_osc52(text);
    }
    #[cfg(test)]
    {
        let _ = text;
    }
}

/// Read the system clipboard. `None` when no native clipboard is reachable;
/// OSC 52 has no read path, so there is no fallback here. Available for a
/// future paste-from-clipboard path; yank only uses the write side today.
#[allow(dead_code)]
pub(crate) fn get_system_clipboard() -> Option<String> {
    #[cfg(not(test))]
    {
        with_native(|cb| cb.get_text().ok()).flatten()
    }
    #[cfg(test)]
    {
        None
    }
}

#[cfg(not(test))]
mod native {
    use std::cell::RefCell;

    thread_local! {
        /// One clipboard handle per editing thread, kept alive for the thread's
        /// lifetime so X11/Wayland selection ownership survives between yanks.
        /// `None` once we've determined no native clipboard is reachable.
        static HANDLE: RefCell<Option<arboard::Clipboard>> =
            RefCell::new(arboard::Clipboard::new().ok());
    }

    /// Run `f` against the long-lived clipboard handle, if one exists.
    pub(super) fn with_native<R>(f: impl FnOnce(&mut arboard::Clipboard) -> R) -> Option<R> {
        HANDLE.with(|cell| cell.borrow_mut().as_mut().map(f))
    }
}

#[cfg(not(test))]
use native::with_native;

/// Write to the native clipboard via the long-lived handle. Returns `false`
/// (so the caller falls back to OSC 52) when no native clipboard is reachable.
#[cfg(not(test))]
fn set_native(text: &str) -> bool {
    with_native(|cb| cb.set_text(text.to_owned()).is_ok()).unwrap_or(false)
}

/// Write the OSC 52 "set clipboard" escape sequence to stdout.
#[cfg(not(test))]
fn emit_osc52(text: &str) {
    use std::io::Write;
    let seq = osc52_sequence(text);
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(seq.as_bytes());
    let _ = stdout.flush();
}

/// Build the OSC 52 escape sequence that sets the system clipboard to `text`.
///
/// Form: `ESC ] 52 ; c ; <base64(text)> BEL`.
fn osc52_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

/// Standard (RFC 4648) base64. Kept dependency-free so the OSC 52 fallback adds
/// no transitive crates beyond the optional `arboard`.
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn osc52_wraps_base64_in_escape() {
        assert_eq!(osc52_sequence("foo"), "\x1b]52;c;Zm9v\x07");
    }
}
