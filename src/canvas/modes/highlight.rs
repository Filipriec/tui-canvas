// src/canvas/modes/highlight.rs
//! Highlight state definitions for canvas visual/selection modes.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Represents the current highlight/visual selection state.
///
/// This enum is used by the GUI and selection logic to track whether a visual
/// selection is active and its anchor position.
pub enum HighlightState {
    /// No highlighting active.
    #[default]
    Off,
    /// Characterwise selection with an anchor (field_index, char_position).
    Characterwise { anchor: (usize, usize) }, // (field_index, char_position)
    /// Linewise selection anchored at a field index.
    Linewise { anchor_line: usize }, // field_index
}
