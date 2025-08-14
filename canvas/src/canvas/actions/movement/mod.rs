// src/canvas/actions/movement/mod.rs

pub mod word;
pub mod line;
pub mod char;

// Re-export commonly used functions
pub use word::{
    find_next_word_start, find_word_end, find_prev_word_start, find_prev_word_end,
    find_next_WORD_start, find_prev_WORD_start, find_WORD_end, find_prev_WORD_end,
    // Add these new exports:
    find_last_word_start_in_field, find_last_word_end_in_field,
    find_last_WORD_start_in_field, find_last_WORD_end_in_field,
};
pub use line::{line_start_position, line_end_position, safe_cursor_position};
pub use char::{move_left, move_right, is_valid_cursor_position, clamp_cursor_position};
