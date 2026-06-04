// src/editor/movement.rs

use crate::canvas::actions::movement::line::{line_end_position, line_start_position};
use crate::canvas::actions::movement::word::{
    find_last_big_word_start_in_field, find_last_word_start_in_field,
};
use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    /// Move cursor left within current field (mask-aware)
    pub fn move_left(&mut self) -> anyhow::Result<()> {
        self.break_undo_coalescing();

        #[cfg(feature = "validation")]
        let mut moved = false;
        #[cfg(not(feature = "validation"))]
        let moved = false;

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    if let Some(prev_input) = mask.prev_input_position(display_pos) {
                        let raw_pos = mask.display_pos_to_raw_pos(prev_input);
                        let max_pos = self.current_text().chars().count();
                        self.set_cursor_raw(raw_pos.min(max_pos));
                        moved = true;
                    } else {
                        self.set_cursor_raw(0);
                        moved = true;
                    }
                }
            }
        }

        if !moved && self.ui_state.cursor_pos > 0 {
            self.set_cursor_raw(self.ui_state.cursor_pos - 1);
        }
        Ok(())
    }

    /// Move cursor right within current field (mask-aware)
    pub fn move_right(&mut self) -> anyhow::Result<()> {
        self.break_undo_coalescing();

        #[cfg(feature = "validation")]
        let mut moved = false;
        #[cfg(not(feature = "validation"))]
        let moved = false;

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    let next_display_pos = mask.next_input_position(display_pos);
                    let next_pos = mask.display_pos_to_raw_pos(next_display_pos);
                    let max_pos = self.current_text().chars().count();
                    self.set_cursor_raw(next_pos.min(max_pos));
                    moved = true;
                }
            }
        }

        if !moved {
            let max_pos = self.current_text().chars().count();
            if self.ui_state.cursor_pos < max_pos {
                self.set_cursor_raw(self.ui_state.cursor_pos + 1);
            }
        }
        Ok(())
    }

    /// Move to start of current field (vim 0)
    pub fn move_line_start(&mut self) {
        let new_pos = line_start_position();
        self.set_cursor_raw(new_pos);
    }

    /// Move to end of current field (vim $)
    pub fn move_line_end(&mut self) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Ins;

        let new_pos = line_end_position(current_text, is_edit_mode);
        self.set_cursor_raw(new_pos);
    }

    /// Set cursor to exact position (for f/F/t/T etc.)
    pub fn set_cursor_position(&mut self, position: usize) {
        let current_text = self.current_text();
        let char_len = current_text.chars().count();
        self.set_cursor_for_mode(position, char_len);
    }
}

impl<D: DataProvider> FormEditor<D> {
    fn move_up_to_previous_field_and_set_last<F>(&mut self, mut position_for_field: F) -> bool
    where
        F: FnMut(&str) -> usize,
    {
        let current_field = self.ui_state.current_field;
        if !self.move_up() || self.ui_state.current_field == current_field {
            return false;
        }

        let new_text = self.current_text();
        if !new_text.is_empty() {
            let pos = position_for_field(new_text);
            self.set_cursor_raw(pos);
        }
        true
    }

    fn move_down_to_next_field_and_set<F>(
        &mut self,
        set_zero_when_empty: bool,
        mut position_for_field: F,
    ) -> bool
    where
        F: FnMut(&str) -> usize,
    {
        if !self.move_down() {
            return false;
        }

        let new_text = self.current_text();
        if new_text.is_empty() {
            if set_zero_when_empty {
                self.set_cursor_raw(0);
            }
        } else {
            let pos = position_for_field(new_text);
            let char_len = new_text.chars().count();
            self.set_cursor_for_mode(pos, char_len);
        }
        true
    }

    /// Move to start of next word (vim w) - can cross field boundaries
    pub fn move_word_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_down_to_next_field_and_set(false, |new_text| {
                if new_text.chars().next().is_some_and(|c| !c.is_whitespace()) {
                    0
                } else {
                    find_next_word_start(new_text, 0)
                }
            });
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_next_word_start(current_text, current_pos);

        if new_pos >= current_text.chars().count() {
            self.move_down_to_next_field_and_set(true, |new_text| {
                if new_text.chars().next().is_some_and(|c| !c.is_whitespace()) {
                    0
                } else {
                    find_next_word_start(new_text, 0)
                }
            });
        } else {
            let char_len = current_text.chars().count();
            self.set_cursor_for_mode(new_pos, char_len);
        }
    }

    /// Move to start of previous word (vim b) - can cross field boundaries
    pub fn move_word_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_up_to_previous_field_and_set_last(find_last_word_start_in_field);
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        if current_pos == 0 {
            self.move_up_to_previous_field_and_set_last(find_last_word_start_in_field);
            return;
        }

        let new_pos = find_prev_word_start(current_text, current_pos);

        if new_pos < current_pos {
            self.set_cursor_raw(new_pos);
        } else {
            self.move_up_to_previous_field_and_set_last(find_last_word_start_in_field);
        }
    }

    /// Move to end of current/next word (vim e) - can cross field boundaries
    pub fn move_word_end(&mut self) {
        use crate::canvas::actions::movement::word::find_word_end;
        let current_text = self.current_text();
        let char_len = current_text.chars().count();
        let current_pos = self.ui_state.cursor_pos;

        if current_text.is_empty() {
            if self.move_down() {
                self.set_cursor_raw(0);
            }
            return;
        }

        let mut target_pos = find_word_end(current_text, current_pos);

        if target_pos <= current_pos && current_pos + 1 < char_len {
            target_pos = find_word_end(current_text, current_pos + 1);
        }

        if target_pos > current_pos {
            self.set_cursor_for_mode(target_pos, char_len);
        } else {
            if self.move_down() {
                self.set_cursor_raw(0);

                let next_text = self.current_text();
                if !next_text.is_empty() {
                    let first_word_end = find_word_end(next_text, 0);
                    let next_char_len = next_text.chars().count();
                    self.set_cursor_for_mode(first_word_end, next_char_len);
                }
            }
        }
    }

    /// Move to end of previous word (vim ge) - can cross field boundaries
    pub fn move_word_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::{
            find_last_word_end_in_field, find_prev_word_end,
        };
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_up_to_previous_field_and_set_last(find_last_word_end_in_field);
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        if current_pos == 0 {
            self.move_up_to_previous_field_and_set_last(find_last_word_end_in_field);
            return;
        }

        let new_pos = find_prev_word_end(current_text, current_pos);

        if new_pos == current_pos {
            self.move_up_to_previous_field_and_set_last(find_last_word_end_in_field);
        } else {
            let char_len = current_text.chars().count();
            self.set_cursor_for_mode(new_pos, char_len);
        }
    }

    /// Move to start of next big_word (vim W) - can cross field boundaries
    pub fn move_big_word_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_big_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_down_to_next_field_and_set(false, |new_text| {
                if new_text.chars().next().is_some_and(|c| !c.is_whitespace()) {
                    0
                } else {
                    find_next_big_word_start(new_text, 0)
                }
            });
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_next_big_word_start(current_text, current_pos);

        if new_pos >= current_text.chars().count() {
            self.move_down_to_next_field_and_set(true, |new_text| {
                if new_text.chars().next().is_some_and(|c| !c.is_whitespace()) {
                    0
                } else {
                    find_next_big_word_start(new_text, 0)
                }
            });
        } else {
            let char_len = current_text.chars().count();
            self.set_cursor_for_mode(new_pos, char_len);
        }
    }

    /// Move to start of previous big_word (vim B) - can cross field boundaries
    pub fn move_big_word_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_big_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_up_to_previous_field_and_set_last(find_last_big_word_start_in_field);
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        if current_pos == 0 {
            self.move_up_to_previous_field_and_set_last(find_last_big_word_start_in_field);
            return;
        }

        let new_pos = find_prev_big_word_start(current_text, current_pos);

        if new_pos < current_pos {
            self.set_cursor_raw(new_pos);
        } else {
            self.move_up_to_previous_field_and_set_last(find_last_big_word_start_in_field);
        }
    }

    /// Move to end of current/next big_word (vim E) - can cross field boundaries
    pub fn move_big_word_end(&mut self) {
        use crate::canvas::actions::movement::word::find_big_word_end;
        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_down_to_next_field_and_set(false, |new_text| find_big_word_end(new_text, 0));
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let char_len = current_text.chars().count();
        let new_pos = find_big_word_end(current_text, current_pos);

        if new_pos == current_pos && current_pos + 1 < char_len {
            let next_pos = find_big_word_end(current_text, current_pos + 1);
            if next_pos < char_len {
                self.set_cursor_for_mode(next_pos, char_len);
                return;
            }
        }

        if new_pos >= char_len.saturating_sub(1) {
            self.move_down_to_next_field_and_set(false, |new_text| find_big_word_end(new_text, 0));
        } else {
            self.set_cursor_for_mode(new_pos, char_len);
        }
    }

    /// Move to end of previous big_word (vim gE) - can cross field boundaries
    pub fn move_big_word_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::{find_big_word_end, find_prev_big_word_end};

        let current_text = self.current_text();

        if current_text.is_empty() {
            self.move_up_to_previous_field_and_set_last(|new_text| find_big_word_end(new_text, 0));
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_prev_big_word_end(current_text, current_pos);

        if new_pos == current_pos {
            self.move_up_to_previous_field_and_set_last(|new_text| find_big_word_end(new_text, 0));
        } else {
            let char_len = current_text.chars().count();
            self.set_cursor_for_mode(new_pos, char_len);
        }
    }
}
