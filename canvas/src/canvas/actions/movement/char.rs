// src/canvas/actions/movement/char.rs

/// Calculate new position when moving left
pub fn move_left(current_pos: usize) -> usize {
    current_pos.saturating_sub(1)
}

/// Calculate new position when moving right
pub fn move_right(current_pos: usize, text: &str, for_edit_mode: bool) -> usize {
    if text.is_empty() {
        return current_pos;
    }

    if for_edit_mode {
        // Edit mode: can move past end of text
        (current_pos + 1).min(text.len())
    } else {
        // Read-only/highlight mode: stays within text bounds
        if current_pos < text.len().saturating_sub(1) {
            current_pos + 1
        } else {
            current_pos
        }
    }
}

/// Check if cursor position is valid for the given mode
pub fn is_valid_cursor_position(pos: usize, text: &str, for_edit_mode: bool) -> bool {
    if text.is_empty() {
        return pos == 0;
    }

    if for_edit_mode {
        pos <= text.len()
    } else {
        pos < text.len()
    }
}

/// Clamp cursor position to valid bounds for the given mode
pub fn clamp_cursor_position(pos: usize, text: &str, for_edit_mode: bool) -> usize {
    if text.is_empty() {
        0
    } else if for_edit_mode {
        pos.min(text.len())
    } else {
        pos.min(text.len().saturating_sub(1))
    }
}
