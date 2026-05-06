// src/canvas/cursor.rs
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
            let style = match mode {
                AppMode::Edit => SetCursorStyle::SteadyBar, // Thin line for insert
                AppMode::ReadOnly => SetCursorStyle::SteadyBlock, // Block for normal
                AppMode::Highlight => SetCursorStyle::BlinkingBlock, // Blinking for visual
                AppMode::General => SetCursorStyle::SteadyBlock, // Block for general
                AppMode::Command => SetCursorStyle::SteadyUnderScore, // Underscore for command
            };

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
