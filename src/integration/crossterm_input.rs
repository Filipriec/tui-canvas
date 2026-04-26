//! Crossterm-specific input setup helpers.
//!
//! These adapters are intentionally isolated in `integration` because they
//! configure process-global terminal behavior and therefore belong at the host
//! application boundary rather than inside editor state constructors.

#[cfg(feature = "crossterm")]
use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
};
#[cfg(feature = "crossterm")]
use std::io;

/// RAII guard that enables bracketed paste for the current terminal session.
///
/// This is the closest the library can get to "automatic" paste support
/// without taking ownership of the host application's terminal lifecycle.
///
/// TODO: Replace this crossterm-specific guard with a backend-agnostic input
/// session abstraction once the crate supports multiple terminal backends.
#[cfg(feature = "crossterm")]
#[derive(Debug)]
pub struct CrosstermInputGuard {
    bracketed_paste_enabled: bool,
}

#[cfg(feature = "crossterm")]
impl CrosstermInputGuard {
    /// Enable bracketed paste and return a guard that disables it on drop.
    pub fn install() -> io::Result<Self> {
        execute!(io::stdout(), EnableBracketedPaste)?;
        Ok(Self {
            bracketed_paste_enabled: true,
        })
    }

    /// Disable bracketed paste immediately.
    pub fn uninstall(&mut self) -> io::Result<()> {
        if self.bracketed_paste_enabled {
            execute!(io::stdout(), DisableBracketedPaste)?;
            self.bracketed_paste_enabled = false;
        }
        Ok(())
    }
}

#[cfg(feature = "crossterm")]
impl Drop for CrosstermInputGuard {
    fn drop(&mut self) {
        let _ = self.uninstall();
    }
}
