// src/modes/handlers/mode_manager.rs
// canvas/src/modes/manager.rs

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    General,   // For intro and admin screens
    ReadOnly,  // Canvas read-only mode
    Edit,      // Canvas edit mode
    Highlight, // Canvas highlight/visual mode
    Command,   // Command mode overlay
}

pub struct ModeManager;

impl ModeManager {
    // Mode transition rules
    pub fn can_enter_command_mode(current_mode: AppMode) -> bool {
        !matches!(current_mode, AppMode::Edit)
    }

    pub fn can_enter_edit_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::ReadOnly)
    }

    pub fn can_enter_read_only_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::Edit | AppMode::Command | AppMode::Highlight)
    }

    pub fn can_enter_highlight_mode(current_mode: AppMode) -> bool {
        matches!(current_mode, AppMode::ReadOnly)
    }


    /// Transition to new mode with automatic cursor update (when cursor-style feature enabled)
    pub fn transition_to_mode(current_mode: AppMode, new_mode: AppMode) -> std::io::Result<AppMode> {
        #[cfg(feature = "textmode-normal")]
        {
            // Always force Edit in normalmode
            return Ok(AppMode::Edit);
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

    /// Enter highlight mode with cursor styling
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

    /// Exit highlight mode with cursor styling
    pub fn exit_highlight_mode_with_cursor() -> std::io::Result<AppMode> {
        let new_mode = AppMode::ReadOnly;
        #[cfg(feature = "cursor-style")]
        {
            let _ = CursorManager::update_for_mode(new_mode);
        }
        Ok(new_mode)
    }
}
