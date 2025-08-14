// src/editor/movement.rs

use crate::canvas::actions::movement::line::{
    line_end_position, line_start_position,
};
use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;
use crate::canvas::actions::movement::word::{
    find_last_WORD_end_in_field, find_last_WORD_start_in_field,
    find_last_word_end_in_field, find_last_word_start_in_field,
    find_next_WORD_start, find_next_word_start, find_prev_WORD_end,
    find_prev_WORD_start, find_prev_word_end, find_prev_word_start,
    find_WORD_end, find_word_end,
};

impl<D: DataProvider> FormEditor<D> {
    /// Move cursor left within current field (mask-aware)
    pub fn move_left(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "validation")]
        let mut moved = false;
        #[cfg(not(feature = "validation"))]
        let moved = false;

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) =
                self.ui_state.validation.get_field_config(field_index)
            {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos =
                        mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    if let Some(prev_input) =
                        mask.prev_input_position(display_pos)
                    {
                        let raw_pos =
                            mask.display_pos_to_raw_pos(prev_input);
                        let max_pos = self.current_text().chars().count();
                        self.ui_state.cursor_pos = raw_pos.min(max_pos);
                        self.ui_state.ideal_cursor_column =
                            self.ui_state.cursor_pos;
                        moved = true;
                    } else {
                        self.ui_state.cursor_pos = 0;
                        self.ui_state.ideal_cursor_column = 0;
                        moved = true;
                    }
                }
            }
        }

        if !moved {
            if self.ui_state.cursor_pos > 0 {
                self.ui_state.cursor_pos -= 1;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }
        Ok(())
    }

    /// Move cursor right within current field (mask-aware)
    pub fn move_right(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "validation")]
        let mut moved = false;
        #[cfg(not(feature = "validation"))]
        let moved = false;

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) =
                self.ui_state.validation.get_field_config(field_index)
            {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos =
                        mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    let next_display_pos = mask.next_input_position(display_pos);
                    let next_pos =
                        mask.display_pos_to_raw_pos(next_display_pos);
                    let max_pos = self.current_text().chars().count();
                    self.ui_state.cursor_pos = next_pos.min(max_pos);
                    self.ui_state.ideal_cursor_column =
                        self.ui_state.cursor_pos;
                    moved = true;
                }
            }
        }

        if !moved {
            let max_pos = self.current_text().chars().count();
            if self.ui_state.cursor_pos < max_pos {
                self.ui_state.cursor_pos += 1;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }
        Ok(())
    }

    /// Move to start of current field (vim 0)
    pub fn move_line_start(&mut self) {
        let new_pos = line_start_position();
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to end of current field (vim $)
    pub fn move_line_end(&mut self) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        let new_pos = line_end_position(current_text, is_edit_mode);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Set cursor to exact position (for f/F/t/T etc.)
    pub fn set_cursor_position(&mut self, position: usize) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        let char_len = current_text.chars().count();
        let max_pos = if is_edit_mode {
            char_len
        } else {
            char_len.saturating_sub(1)
        };

        let clamped_pos = position.min(max_pos);
        self.ui_state.cursor_pos = clamped_pos;
        self.ui_state.ideal_cursor_column = clamped_pos;
    }
}


impl<D: DataProvider> FormEditor<D> {
    /// Move to start of next word (vim w) - can cross field boundaries
    pub fn move_word_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to next field
            if self.move_down().is_ok() {
                // Successfully moved to next field, try to find first word
                let new_text = self.current_text();
                if !new_text.is_empty() {
                    let first_word_pos = if new_text.chars().next().map_or(false, |c| !c.is_whitespace()) {
                        // Field starts with non-whitespace, go to position 0
                        0
                    } else {
                        // Field starts with whitespace, find first word
                        find_next_word_start(new_text, 0)
                    };
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_word_pos.min(char_len)
                    } else {
                        first_word_pos.min(char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_next_word_start(current_text, current_pos);
        
        // Check if we've hit the end of the current field
        if new_pos >= current_text.chars().count() {
            // At end of field - jump to next field and start from beginning
            if self.move_down().is_ok() {
                // Successfully moved to next field
                let new_text = self.current_text();
                if new_text.is_empty() {
                    // New field is empty, cursor stays at 0
                    self.ui_state.cursor_pos = 0;
                    self.ui_state.ideal_cursor_column = 0;
                } else {
                    // Find first word in new field
                    let first_word_pos = if new_text.chars().next().map_or(false, |c| !c.is_whitespace()) {
                        // Field starts with non-whitespace, go to position 0
                        0
                    } else {
                        // Field starts with whitespace, find first word
                        find_next_word_start(new_text, 0)
                    };
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_word_pos.min(char_len)
                    } else {
                        first_word_pos.min(char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
            // If move_down() failed, we stay where we are (at end of last field)
        } else {
            // Normal word movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let char_len = current_text.chars().count();
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }

    /// Move to start of previous word (vim b) - can cross field boundaries
    pub fn move_word_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to previous field and find last word
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_word_start = find_last_word_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_start;
                        self.ui_state.ideal_cursor_column = last_word_start;
                    }
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        // Special case: if we're at position 0, jump to previous field
        if current_pos == 0 {
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_word_start = find_last_word_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_start;
                        self.ui_state.ideal_cursor_column = last_word_start;
                    }
                }
            }
            return;
        }

        // Try to find previous word in current field
        let new_pos = find_prev_word_start(current_text, current_pos);

        // Check if we actually moved
        if new_pos < current_pos {
            // Normal word movement within current field - we found a previous word
            self.ui_state.cursor_pos = new_pos;
            self.ui_state.ideal_cursor_column = new_pos;
        } else {
            // We didn't move (probably at start of first word), try previous field
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_word_start = find_last_word_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_start;
                        self.ui_state.ideal_cursor_column = last_word_start;
                    }
                }
            }
        }
    }

    /// Move to end of current/next word (vim e) - can cross field boundaries
    pub fn move_word_end(&mut self) {
        use crate::canvas::actions::movement::word::find_word_end;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to next field
            if self.move_down().is_ok() {
                // Recursively call move_word_end in the new field
                self.move_word_end();
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let char_len = current_text.chars().count();
        let new_pos = find_word_end(current_text, current_pos);

        // Check if we didn't move or hit the end of the field
        if new_pos == current_pos && current_pos + 1 < char_len {
            // Try next character and find word end from there
            let next_pos = find_word_end(current_text, current_pos + 1);
            if next_pos < char_len {
                let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                let final_pos = if is_edit_mode {
                    next_pos.min(char_len)
                } else {
                    next_pos.min(char_len.saturating_sub(1))
                };
                self.ui_state.cursor_pos = final_pos;
                self.ui_state.ideal_cursor_column = final_pos;
                return;
            }
        }

        // If we're at or near the end of the field, try next field
        if new_pos >= char_len.saturating_sub(1) {
            if self.move_down().is_ok() {
                // Position at start and find first word end
                self.ui_state.cursor_pos = 0;
                self.ui_state.ideal_cursor_column = 0;
                self.move_word_end();
            }
        } else {
            // Normal word end movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }

    /// Move to end of previous word (vim ge) - can cross field boundaries
    pub fn move_word_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::{find_prev_word_end, find_last_word_end_in_field};
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to previous field (but don't recurse)
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        // Find end of last word in the field
                        let last_word_end = find_last_word_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_end;
                        self.ui_state.ideal_cursor_column = last_word_end;
                    }
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        // Special case: if we're at position 0, jump to previous field (but don't recurse)
        if current_pos == 0 {
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_word_end = find_last_word_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_end;
                        self.ui_state.ideal_cursor_column = last_word_end;
                    }
                }
            }
            return;
        }

        // CHANGE THIS LINE: replace find_prev_word_end_corrected with find_prev_word_end
        let new_pos = find_prev_word_end(current_text, current_pos);

        // Only try to cross fields if we didn't move at all (stayed at same position)
        if new_pos == current_pos {
            // We didn't move within the current field, try previous field
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_word_end = find_last_word_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_word_end;
                        self.ui_state.ideal_cursor_column = last_word_end;
                    }
                }
            }
        } else {
            // Normal word movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let char_len = current_text.chars().count();
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }

    /// Move to start of next WORD (vim W) - can cross field boundaries
    pub fn move_WORD_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_WORD_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to next field
            if self.move_down().is_ok() {
                // Successfully moved to next field, try to find first WORD
                let new_text = self.current_text();
                if !new_text.is_empty() {
                    let first_WORD_pos = if new_text.chars().next().map_or(false, |c| !c.is_whitespace()) {
                        // Field starts with non-whitespace, go to position 0
                        0
                    } else {
                        // Field starts with whitespace, find first WORD
                        find_next_WORD_start(new_text, 0)
                    };
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_WORD_pos.min(char_len)
                    } else {
                        first_WORD_pos.min(char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_next_WORD_start(current_text, current_pos);

        // Check if we've hit the end of the current field
        if new_pos >= current_text.chars().count() {
            // At end of field - jump to next field and start from beginning
            if self.move_down().is_ok() {
                // Successfully moved to next field
                let new_text = self.current_text();
                if new_text.is_empty() {
                    // New field is empty, cursor stays at 0
                    self.ui_state.cursor_pos = 0;
                    self.ui_state.ideal_cursor_column = 0;
                } else {
                    // Find first WORD in new field
                    let first_WORD_pos = if new_text.chars().next().map_or(false, |c| !c.is_whitespace()) {
                        // Field starts with non-whitespace, go to position 0
                        0
                    } else {
                        // Field starts with whitespace, find first WORD
                        find_next_WORD_start(new_text, 0)
                    };
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_WORD_pos.min(char_len)
                    } else {
                        first_WORD_pos.min(char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
            // If move_down() failed, we stay where we are (at end of last field)
        } else {
            // Normal WORD movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let char_len = current_text.chars().count();
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }

    /// Move to start of previous WORD (vim B) - can cross field boundaries
    pub fn move_WORD_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_WORD_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to previous field and find last WORD
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_WORD_start = find_last_WORD_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_start;
                        self.ui_state.ideal_cursor_column = last_WORD_start;
                    }
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        // Special case: if we're at position 0, jump to previous field
        if current_pos == 0 {
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_WORD_start = find_last_WORD_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_start;
                        self.ui_state.ideal_cursor_column = last_WORD_start;
                    }
                }
            }
            return;
        }

        // Try to find previous WORD in current field
        let new_pos = find_prev_WORD_start(current_text, current_pos);

        // Check if we actually moved
        if new_pos < current_pos {
            // Normal WORD movement within current field - we found a previous WORD
            self.ui_state.cursor_pos = new_pos;
            self.ui_state.ideal_cursor_column = new_pos;
        } else {
            // We didn't move (probably at start of first WORD), try previous field
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_WORD_start = find_last_WORD_start_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_start;
                        self.ui_state.ideal_cursor_column = last_WORD_start;
                    }
                }
            }
        }
    }

    /// Move to end of current/next WORD (vim E) - can cross field boundaries
    pub fn move_WORD_end(&mut self) {
        use crate::canvas::actions::movement::word::find_WORD_end;
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to next field (but don't recurse)
            if self.move_down().is_ok() {
                let new_text = self.current_text();
                if !new_text.is_empty() {
                    // Find first WORD end in new field
                    let first_WORD_end = find_WORD_end(new_text, 0);
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_WORD_end.min(char_len)
                    } else {
                        first_WORD_end.min(char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let char_len = current_text.chars().count();
        let new_pos = find_WORD_end(current_text, current_pos);

        // Check if we didn't move or hit the end of the field
        if new_pos == current_pos && current_pos + 1 < char_len {
            // Try next character and find WORD end from there
            let next_pos = find_WORD_end(current_text, current_pos + 1);
            if next_pos < char_len {
                let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                let final_pos = if is_edit_mode {
                    next_pos.min(char_len)
                } else {
                    next_pos.min(char_len.saturating_sub(1))
                };
                self.ui_state.cursor_pos = final_pos;
                self.ui_state.ideal_cursor_column = final_pos;
                return;
            }
        }

        // If we're at or near the end of the field, try next field (but don't recurse)
        if new_pos >= char_len.saturating_sub(1) {
            if self.move_down().is_ok() {
                // Find first WORD end in new field
                let new_text = self.current_text();
                if !new_text.is_empty() {
                    let first_WORD_end = find_WORD_end(new_text, 0);
                    let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
                    let new_char_len = new_text.chars().count();
                    let final_pos = if is_edit_mode {
                        first_WORD_end.min(new_char_len)
                    } else {
                        first_WORD_end.min(new_char_len.saturating_sub(1))
                    };
                    self.ui_state.cursor_pos = final_pos;
                    self.ui_state.ideal_cursor_column = final_pos;
                }
            }
        } else {
            // Normal WORD end movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }

    /// Move to end of previous WORD (vim gE) - can cross field boundaries
    pub fn move_WORD_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::{find_prev_WORD_end, find_WORD_end};
        let current_text = self.current_text();

        if current_text.is_empty() {
            // Empty field - try to move to previous field (but don't recurse)
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        // Find end of last WORD in the field
                        let last_WORD_end = find_last_WORD_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_end;
                        self.ui_state.ideal_cursor_column = last_WORD_end;
                    }
                }
            }
            return;
        }

        let current_pos = self.ui_state.cursor_pos;

        // Special case: if we're at position 0, jump to previous field (but don't recurse)
        if current_pos == 0 {
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_WORD_end = find_last_WORD_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_end;
                        self.ui_state.ideal_cursor_column = last_WORD_end;
                    }
                }
            }
            return;
        }

        let new_pos = find_prev_WORD_end(current_text, current_pos);

        // Only try to cross fields if we didn't move at all (stayed at same position)
        if new_pos == current_pos {
            // We didn't move within the current field, try previous field
            let current_field = self.ui_state.current_field;
            if self.move_up().is_ok() {
                // Check if we actually moved to a different field
                if self.ui_state.current_field != current_field {
                    let new_text = self.current_text();
                    if !new_text.is_empty() {
                        let last_WORD_end = find_last_WORD_end_in_field(new_text);
                        self.ui_state.cursor_pos = last_WORD_end;
                        self.ui_state.ideal_cursor_column = last_WORD_end;
                    }
                }
            }
        } else {
            // Normal WORD movement within current field
            let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
            let char_len = current_text.chars().count();
            let final_pos = if is_edit_mode {
                new_pos.min(char_len)
            } else {
                new_pos.min(char_len.saturating_sub(1))
            };

            self.ui_state.cursor_pos = final_pos;
            self.ui_state.ideal_cursor_column = final_pos;
        }
    }
}
