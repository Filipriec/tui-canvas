// src/editor/editing.rs
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    /// Open new line below (vim o)
    pub fn open_line_below(&mut self) -> anyhow::Result<()> {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }
        let next_field = (self.ui_state.current_field + 1).min(field_count.saturating_sub(1));
        self.transition_to_field(next_field)?;
        self.set_cursor_raw(0);
        self.enter_edit_mode();
        Ok(())
    }

    /// Open new line above (vim O)
    pub fn open_line_above(&mut self) -> anyhow::Result<()> {
        let prev_field = self.ui_state.current_field.saturating_sub(1);
        self.transition_to_field(prev_field)?;
        self.set_cursor_raw(0);
        self.enter_edit_mode();
        Ok(())
    }

    /// Handle character insertion (mask/limit-aware)
    pub fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        if self.ui_state.current_mode != crate::canvas::modes::AppMode::Ins {
            return Ok(());
        }

        #[cfg(feature = "validation")]
        let field_index = self.ui_state.current_field;
        #[cfg(feature = "validation")]
        let raw_cursor_pos = self.ui_state.cursor_pos;
        #[cfg(feature = "validation")]
        let current_raw_text = self.data_provider.field_value(field_index);

        #[cfg(not(feature = "validation"))]
        let field_index = self.ui_state.current_field;
        #[cfg(not(feature = "validation"))]
        let raw_cursor_pos = self.ui_state.cursor_pos;
        #[cfg(not(feature = "validation"))]
        let current_raw_text = self.data_provider.field_value(field_index);

        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_cursor_pos = mask.raw_pos_to_display_pos(raw_cursor_pos);

                    let pattern_char_len = mask.pattern().chars().count();
                    if display_cursor_pos >= pattern_char_len {
                        return Ok(());
                    }

                    if !mask.is_input_position(display_cursor_pos) {
                        return Ok(());
                    }

                    let input_slots = (0..pattern_char_len)
                        .filter(|&pos| mask.is_input_position(pos))
                        .count();
                    if current_raw_text.chars().count() >= input_slots {
                        return Ok(());
                    }
                }
            }
        }

        #[cfg(feature = "validation")]
        {
            let vr = self.ui_state.validation.validate_char_insertion(
                field_index,
                current_raw_text,
                raw_cursor_pos,
                ch,
            );
            if !vr.is_acceptable() {
                return Ok(());
            }
        }

        let new_raw_text = {
            let mut temp = current_raw_text.to_string();
            let byte_pos = Self::char_to_byte_index(current_raw_text, raw_cursor_pos);
            temp.insert(byte_pos, ch);
            temp
        };

        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(limits) = &cfg.character_limits {
                    if let Some(result) = limits.validate_content(&new_raw_text) {
                        if !result.is_acceptable() {
                            return Ok(());
                        }
                    }
                }
                if let Some(mask) = &cfg.display_mask {
                    let pattern_char_len = mask.pattern().chars().count();
                    let input_slots = (0..pattern_char_len)
                        .filter(|&pos| mask.is_input_position(pos))
                        .count();
                    if new_raw_text.chars().count() > input_slots {
                        return Ok(());
                    }
                }
            }
        }

        self.record_checkpoint(crate::editor::features::history::EditKind::Insert);

        self.data_provider
            .set_field_value(field_index, new_raw_text.clone());

        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let new_raw_pos = raw_cursor_pos + 1;
                    let display_pos = mask.raw_pos_to_display_pos(new_raw_pos);
                    let next_input_display = mask.next_input_position(display_pos);
                    let next_raw_pos = mask.display_pos_to_raw_pos(next_input_display);
                    let max_raw = new_raw_text.chars().count();

                    self.set_cursor_raw(next_raw_pos.min(max_raw));
                    return Ok(());
                }
            }
        }

        self.set_cursor_raw(raw_cursor_pos + 1);

        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();

        Ok(())
    }

    /// Insert plain text at the current cursor position.
    ///
    /// This intentionally treats the input as a character stream and reuses
    /// `insert_char` so validation, masking, and suggestions continue to flow
    /// through the existing editing logic.
    pub fn insert_text(&mut self, text: &str) -> anyhow::Result<()> {
        for ch in text.chars() {
            self.insert_char(ch)?;
        }
        Ok(())
    }

    /// Delete backward (backspace)
    pub fn delete_backward(&mut self) -> anyhow::Result<()> {
        if self.ui_state.current_mode != crate::canvas::modes::AppMode::Ins {
            return Ok(());
        }
        if self.ui_state.cursor_pos == 0 {
            return Ok(());
        }

        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();

        let new_cursor = self.ui_state.cursor_pos.saturating_sub(1);

        let start = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos - 1);
        let end = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos);
        current_text.replace_range(start..end, "");

        self.record_checkpoint(crate::editor::features::history::EditKind::Delete);

        self.data_provider
            .set_field_value(field_index, current_text.clone());

        #[cfg(feature = "validation")]
        let mut target_cursor = new_cursor;
        #[cfg(not(feature = "validation"))]
        let target_cursor = new_cursor;

        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(new_cursor);
                    if let Some(prev_input) = mask.prev_input_position(display_pos) {
                        target_cursor = mask.display_pos_to_raw_pos(prev_input);
                    }
                }
            }
        }

        self.set_cursor_raw(target_cursor);

        #[cfg(feature = "validation")]
        {
            let _ = self
                .ui_state
                .validation
                .validate_field_content(field_index, &current_text);
        }

        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();

        Ok(())
    }

    /// Delete forward (Delete key)
    pub fn delete_forward(&mut self) -> anyhow::Result<()> {
        if self.ui_state.current_mode != crate::canvas::modes::AppMode::Ins {
            return Ok(());
        }

        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();

        if self.ui_state.cursor_pos < current_text.chars().count() {
            let start = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos);
            let end = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos + 1);
            current_text.replace_range(start..end, "");

            self.record_checkpoint(crate::editor::features::history::EditKind::Delete);

            self.data_provider
                .set_field_value(field_index, current_text.clone());

            #[cfg(feature = "validation")]
            let mut target_cursor = self.ui_state.cursor_pos;
            #[cfg(not(feature = "validation"))]
            let target_cursor = self.ui_state.cursor_pos;

            #[cfg(feature = "validation")]
            {
                if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                    if let Some(mask) = &cfg.display_mask {
                        let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                        let next_input = mask.next_input_position(display_pos);
                        target_cursor = mask
                            .display_pos_to_raw_pos(next_input)
                            .min(current_text.chars().count());
                    }
                }
            }

            self.set_cursor_raw(target_cursor);

            #[cfg(feature = "validation")]
            {
                let _ = self
                    .ui_state
                    .validation
                    .validate_field_content(field_index, &current_text);
            }

            #[cfg(feature = "suggestions")]
            self.check_suggestion_trigger();
        }

        Ok(())
    }

    /// Enter edit mode with cursor positioned for append (vim 'a')
    pub fn enter_append_mode(&mut self) {
        #[cfg(feature = "keybindings")]
        {
            use crate::editor::behavior::KeybindingParadigm;
            match self.keybinding_paradigm() {
                KeybindingParadigm::Helix => self.enter_append_mode_helix(),
                KeybindingParadigm::Emacs => self.enter_append_mode_emacs(),
                KeybindingParadigm::Vim => self.enter_append_mode_vim(),
            }
            return;
        }

        #[cfg(not(feature = "keybindings"))]
        self.enter_append_mode_vim();
    }

    /// Set current field value (validates under feature flag)
    pub fn set_current_field_value(&mut self, value: String) {
        let field_index = self.ui_state.current_field;

        self.record_checkpoint(crate::editor::features::history::EditKind::Other);

        self.data_provider
            .set_field_value(field_index, value.clone());
        self.set_cursor_raw(0);

        #[cfg(feature = "validation")]
        {
            let _ = self
                .ui_state
                .validation
                .validate_field_content(field_index, &value);
        }
    }

    /// Set specific field value by index (validates under feature flag)
    pub fn set_field_value(&mut self, field_index: usize, value: String) {
        if field_index < self.data_provider.field_count() {
            self.record_checkpoint(crate::editor::features::history::EditKind::Other);

            self.data_provider
                .set_field_value(field_index, value.clone());
            if field_index == self.ui_state.current_field {
                self.set_cursor_raw(0);
            }

            #[cfg(feature = "validation")]
            {
                let _ = self
                    .ui_state
                    .validation
                    .validate_field_content(field_index, &value);
            }
        }
    }

    /// Clear the current field
    pub fn clear_current_field(&mut self) {
        self.set_current_field_value(String::new());
    }
}
