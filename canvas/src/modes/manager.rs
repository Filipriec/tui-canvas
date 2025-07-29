// src/modes/handlers/mode_manager.rs
// canvas/src/modes/manager.rs


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
}
