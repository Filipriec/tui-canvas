//! Crossterm-specific input setup helpers.
//!
//! These adapters are intentionally isolated in `integration` because they
//! configure process-global terminal behavior and therefore belong at the host
//! application boundary rather than inside editor state constructors.

#[cfg(feature = "crossterm")]
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "crossterm")]
use std::{io, time::Duration};

/// Crossterm session options for Canvas-managed terminal input setup.
///
/// Defaults are intentionally focused on input:
/// - raw mode enabled
/// - bracketed paste enabled
/// - alternate screen disabled
/// - mouse capture disabled
#[cfg(feature = "crossterm")]
#[derive(Debug, Clone, Copy)]
pub struct CrosstermInputOptions {
    pub raw_mode: bool,
    pub alternate_screen: bool,
    pub mouse_capture: bool,
    pub bracketed_paste: bool,
}

#[cfg(feature = "crossterm")]
impl Default for CrosstermInputOptions {
    fn default() -> Self {
        Self {
            raw_mode: true,
            alternate_screen: false,
            mouse_capture: false,
            bracketed_paste: true,
        }
    }
}

#[cfg(feature = "crossterm")]
impl CrosstermInputOptions {
    pub fn tui_defaults() -> Self {
        Self {
            alternate_screen: true,
            mouse_capture: true,
            ..Self::default()
        }
    }

    pub fn with_raw_mode(mut self, enabled: bool) -> Self {
        self.raw_mode = enabled;
        self
    }

    pub fn with_alternate_screen(mut self, enabled: bool) -> Self {
        self.alternate_screen = enabled;
        self
    }

    pub fn with_mouse_capture(mut self, enabled: bool) -> Self {
        self.mouse_capture = enabled;
        self
    }

    pub fn with_bracketed_paste(mut self, enabled: bool) -> Self {
        self.bracketed_paste = enabled;
        self
    }
}

/// Canvas-owned crossterm input session.
///
/// This is the intended high-level binding point for host applications that
/// want Canvas text widgets to work smoothly with crossterm out of the box.
///
/// TODO: Replace this crossterm-specific session with a backend-agnostic
/// terminal/session abstraction once the crate supports multiple backends.
#[cfg(feature = "crossterm")]
#[derive(Debug)]
pub struct CrosstermInputSession {
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    mouse_capture_enabled: bool,
    bracketed_paste_enabled: bool,
}

#[cfg(feature = "crossterm")]
impl CrosstermInputSession {
    pub fn install() -> io::Result<Self> {
        Self::install_with_options(CrosstermInputOptions::default())
    }

    pub fn install_with_options(options: CrosstermInputOptions) -> io::Result<Self> {
        let mut session = Self {
            raw_mode_enabled: false,
            alternate_screen_enabled: false,
            mouse_capture_enabled: false,
            bracketed_paste_enabled: false,
        };

        if options.raw_mode {
            enable_raw_mode()?;
            session.raw_mode_enabled = true;
        }

        let install_result = (|| -> io::Result<()> {
            if options.alternate_screen {
                execute!(io::stdout(), EnterAlternateScreen)?;
                session.alternate_screen_enabled = true;
            }

            if options.mouse_capture {
                execute!(io::stdout(), EnableMouseCapture)?;
                session.mouse_capture_enabled = true;
            }

            if options.bracketed_paste {
                execute!(io::stdout(), EnableBracketedPaste)?;
                session.bracketed_paste_enabled = true;
            }

            Ok(())
        })();

        if let Err(err) = install_result {
            let _ = session.uninstall();
            return Err(err);
        }

        Ok(session)
    }

    pub fn read_event(&self) -> io::Result<Event> {
        event::read()
    }

    pub fn poll_event(&self, timeout: Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    pub fn uninstall(&mut self) -> io::Result<()> {
        if self.bracketed_paste_enabled {
            execute!(io::stdout(), DisableBracketedPaste)?;
            self.bracketed_paste_enabled = false;
        }

        if self.mouse_capture_enabled {
            execute!(io::stdout(), DisableMouseCapture)?;
            self.mouse_capture_enabled = false;
        }

        if self.alternate_screen_enabled {
            execute!(io::stdout(), LeaveAlternateScreen)?;
            self.alternate_screen_enabled = false;
        }

        if self.raw_mode_enabled {
            disable_raw_mode()?;
            self.raw_mode_enabled = false;
        }

        Ok(())
    }
}

#[cfg(feature = "crossterm")]
impl Drop for CrosstermInputSession {
    fn drop(&mut self) {
        let _ = self.uninstall();
    }
}

/// RAII guard that enables bracketed paste for the current terminal session.
///
/// This is the closest the library can get to "automatic" paste support
/// without taking ownership of the host application's terminal lifecycle.
///
/// Prefer [`CrosstermInputSession`] for new code when you want Canvas to own
/// the input-related terminal setup more completely.
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
