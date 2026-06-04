// src/editor/display.rs

use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    /// Get current field text for display.
    #[cfg(feature = "validation")]
    pub fn current_display_text(&self) -> String {
        let field_index = self.ui_state.current_field;
        let raw = if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        };

        if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
            if cfg.custom_formatter.is_none() {
                if let Some(mask) = &cfg.display_mask {
                    return mask.apply_to_display(raw);
                }
            }

            if cfg.custom_formatter.is_some() {
                if matches!(self.ui_state.current_mode, AppMode::Ins) {
                    return raw.to_string();
                }
                if let Some((formatted, _mapper, _warning)) = cfg.run_custom_formatter(raw) {
                    return formatted;
                }
            }

            if let Some(mask) = &cfg.display_mask {
                return mask.apply_to_display(raw);
            }
        }

        raw.to_string()
    }

    /// Get effective display text for any field index (Feature 4 + masks).
    #[cfg(feature = "validation")]
    pub fn display_text_for_field(&self, field_index: usize) -> String {
        let raw = if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        };

        if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
            if cfg.custom_formatter.is_none() {
                if let Some(mask) = &cfg.display_mask {
                    return mask.apply_to_display(raw);
                }
            }

            if cfg.custom_formatter.is_some() {
                if field_index == self.ui_state.current_field
                    && matches!(self.ui_state.current_mode, AppMode::Ins)
                {
                    return raw.to_string();
                }
                if let Some((formatted, _mapper, _warning)) = cfg.run_custom_formatter(raw) {
                    return formatted;
                }
            }

            if let Some(mask) = &cfg.display_mask {
                return mask.apply_to_display(raw);
            }
        }

        raw.to_string()
    }

    /// Map raw cursor to display position (formatter/mask aware).
    pub fn display_cursor_position(&self) -> usize {
        let current_text = self.current_text();
        let char_count = current_text.chars().count();

        let raw_pos = match self.ui_state.current_mode {
            AppMode::Ins => self.ui_state.cursor_pos.min(char_count),
            _ => {
                if char_count == 0 {
                    0
                } else {
                    self.ui_state.cursor_pos.min(char_count.saturating_sub(1))
                }
            }
        };

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if !matches!(self.ui_state.current_mode, AppMode::Ins) {
                    if let Some((formatted, mapper, _)) = cfg.run_custom_formatter(current_text) {
                        return mapper.raw_to_formatted(current_text, &formatted, raw_pos);
                    }
                }
                if let Some(mask) = &cfg.display_mask {
                    return mask.raw_pos_to_display_pos(raw_pos);
                }
            }
        }

        raw_pos
    }
}
