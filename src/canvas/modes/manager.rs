// src/canvas/modes/manager.rs
//! App mode definitions used by the canvas editor.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Top-level application modes used by the canvas UI.
///
/// These modes control input handling, cursor behavior, and how the UI should
/// respond to user actions.
pub enum AppMode {
    /// For intro and admin screens
    General,
    /// Canvas insert mode (insertion/modification)
    Ins,
    /// Canvas selection mode (visual selection)
    Sel,
    /// Canvas normal mode (navigation)
    Nor,
    /// Command mode overlay (for commands)
    Command,
}
