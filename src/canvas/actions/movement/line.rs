// src/canvas/actions/movement/line.rs
//! Line-level cursor movement and positioning

/// Calculate cursor position for line start
pub fn line_start_position() -> usize {
    0
}

/// Calculate cursor position for line end
pub fn line_end_position(text: &str, for_edit_mode: bool) -> usize {
    if text.is_empty() {
        0
    } else if for_edit_mode {
        // Cursor can go past end of text
        text.len()
    } else {
        // Read-only/highlight mode: cursor stays on last character
        text.len().saturating_sub(1)
    }
}

/// Calculate safe cursor position when switching fields
pub fn safe_cursor_position(text: &str, ideal_column: usize, for_edit_mode: bool) -> usize {
    if text.is_empty() {
        0
    } else if for_edit_mode {
        // Cursor can go past end
        ideal_column.min(text.len())
    } else {
        // Read-only/highlight mode: cursor stays within text
        ideal_column.min(text.len().saturating_sub(1))
    }
}
