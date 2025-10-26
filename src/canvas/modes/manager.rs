// src/modes/handlers/mode_manager.rs
// canvas/src/modes/manager.rs
//! Mode manager utilities and the AppMode enum.
//!
//! This module defines the available canvas modes and provides helper
//! functions to validate mode transitions and perform required side-effects
//! such as updating cursor style when enabled.

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Top-level application modes used by the canvas UI.
///
/// These modes control input handling, cursor behavior, and how the UI should
/// respond to user actions.
pub enum AppMode {
    /// For intro and admin screens
    General,
    /// Canvas read-only mode (navigation)
    ReadOnly,
    /// Canvas edit mode (insertion/modification)
    Edit,
    /// Canvas highlight/visual mode (selection)
    Highlight,
    /// Command mode overlay (for commands)
    Command,
}

pub struct ModeManager;

impl ModeManager {
    // Mode transition rules

    /// Return true if the system can enter Command mode from the given current mode.
    pub fn can_enter_command_mode(current_mode: AppMode) -> bool {
        !matches!(current_mode, AppMode::Edit)
    }

    /// Return true if the system can enter Edit mode from the given current mode.
    pub fn can_enter_edit_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::ReadOnly)
    }

    /// Return true if the system can enter ReadOnly mode from the given current mode.
    pub fn can_enter_read_only_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::Edit | AppMode::Command | AppMode::Highlight)
    }

    /// Return true if the system can enter Highlight mode from the given current mode.
    pub fn can_enter_highlight_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::ReadOnly)
    }


    /// Transition to new mode with automatic cursor update (when cursor-style feature enabled).
    ///
    /// Returns the resulting mode or an I/O error if cursor style update fails.
    pub fn transition_to_mode(current_mode: AppMode, new_mode: AppMode) -> std::io::Result<AppMode> {
        #[cfg(feature = "textmode-normal")]
        {
            // Always force Edit in normalmode
            Ok(AppMode::Edit)
        }

        #[cfg(not(feature = "textmode-normal"))]
        {
            if current_mode != new_mode {
                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(new_mode);
                }
            }
            Ok(new_mode)
        }
    }

    /// Enter highlight mode with cursor styling.
    ///
    /// Returns Ok(true) if the transition succeeded (and cursor style was updated
    /// when enabled), otherwise Ok(false) if the transition is not allowed.
    pub fn enter_highlight_mode_with_cursor(current_mode: AppMode) -> std::io::Result<bool> {
        if Self::can_enter_highlight_mode(current_mode) {
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Highlight);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Exit highlight mode with cursor styling and return the next mode.
    ///
    /// This helper returns the mode to switch to (ReadOnly) and updates cursor
    /// style if the feature is enabled.
    pub fn exit_highlight_mode_with_cursor() -> std::io::Result<AppMode> {
        let new_mode = AppMode::ReadOnly;
        #[cfg(feature = "cursor-style")]
        {
            let _ = CursorManager::update_for_mode(new_mode);
        }
        Ok(new_mode)
    }
}
