// src/canvas/actions/movement/mod.rs
//! Movement utilities for character, word, and line navigation

pub mod char;
pub mod line;
pub mod word;

// Re-export commonly used functions
pub use char::{clamp_cursor_position, is_valid_cursor_position, move_left, move_right};
pub use line::{line_end_position, line_start_position, safe_cursor_position};
pub use word::{
    find_big_word_end, find_last_big_word_end_in_field, find_last_big_word_start_in_field,
    find_last_word_end_in_field, find_last_word_start_in_field, find_next_big_word_start,
    find_next_word_start, find_prev_big_word_end, find_prev_big_word_start, find_prev_word_end,
    find_prev_word_start, find_word_end,
};
