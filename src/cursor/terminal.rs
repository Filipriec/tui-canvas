// src/cursor/terminal.rs
//! Cursor style management for different canvas modes
//!
//! Provides helpers to update and reset terminal cursor style when the
//! `cursor-style` feature is enabled. When the feature is disabled the
//! functions are no-ops.

#[cfg(feature = "cursor-style")]
use crossterm::{cursor::SetCursorStyle, execute};
#[cfg(feature = "cursor-style")]
use std::io;

use crate::canvas::modes::AppMode;

#[cfg(feature = "cursor-style")]
fn cursor_style_for_mode(mode: AppMode) -> SetCursorStyle {
    match mode {
        AppMode::Ins => SetCursorStyle::SteadyBlock,
        AppMode::Nor => SetCursorStyle::SteadyBlock,
        AppMode::Sel => SetCursorStyle::BlinkingBlock,
        AppMode::General => SetCursorStyle::SteadyBlock,
        AppMode::Command => SetCursorStyle::SteadyUnderScore,
    }
}

/// Manages cursor styles based on canvas modes
pub struct CursorManager;

impl CursorManager {
    /// Update cursor style based on current mode.
    ///
    /// When the `textmode-normal` feature is enabled a fixed style is applied.
    /// Otherwise, the cursor style is mapped to the provided AppMode.
    #[cfg(feature = "cursor-style")]
    pub fn update_for_mode(mode: AppMode) -> io::Result<()> {
        // NORMALMODE: force underscore for every mode
        #[cfg(feature = "textmode-normal")]
        {
            let style = SetCursorStyle::SteadyBar;
            execute!(io::stdout(), style)
        }

        // Default (not normal): original mapping
        #[cfg(not(feature = "textmode-normal"))]
        {
            let style = cursor_style_for_mode(mode);

            return execute!(io::stdout(), style);
        }
    }

    /// No-op when cursor-style feature is disabled.
    #[cfg(not(feature = "cursor-style"))]
    pub fn update_for_mode(_mode: AppMode) -> io::Result<()> {
        Ok(())
    }

    /// Reset cursor to default on cleanup.
    #[cfg(feature = "cursor-style")]
    pub fn reset() -> io::Result<()> {
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape)
    }

    /// Reset is a no-op when the cursor-style feature is disabled.
    #[cfg(not(feature = "cursor-style"))]
    pub fn reset() -> io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, feature = "cursor-style"))]
mod tests {
    use super::*;
    use crossterm::Command;

    fn ansi_for_mode(mode: AppMode) -> String {
        let mut out = String::new();
        cursor_style_for_mode(mode).write_ansi(&mut out).unwrap();
        out
    }

    #[test]
    fn insert_mode_uses_block_cursor_shape() {
        assert_eq!(ansi_for_mode(AppMode::Ins), "\x1b[2 q");
    }
}
