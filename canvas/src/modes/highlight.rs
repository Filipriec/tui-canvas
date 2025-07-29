// src/state/app/highlight.rs
// canvas/src/modes/highlight.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightState {
    Off,
    Characterwise { anchor: (usize, usize) }, // (field_index, char_position)
    Linewise { anchor_line: usize },          // field_index
}

impl Default for HighlightState {
    fn default() -> Self {
        HighlightState::Off
    }
}
