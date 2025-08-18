// src/state/app/highlight.rs
// canvas/src/modes/highlight.rs

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum HighlightState {
    #[default]
    Off,
    Characterwise { anchor: (usize, usize) }, // (field_index, char_position)
    Linewise { anchor_line: usize },          // field_index
}

