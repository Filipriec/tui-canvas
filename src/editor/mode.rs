// src/editor/mode.rs

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    /// Change mode
    pub fn set_mode(&mut self, mode: AppMode) {
        // Avoid unused param warning in normalmode
        #[cfg(feature = "textmode-normal")]
        let _ = mode;

        // NORMALMODE: force Edit, ignore requested mode
        #[cfg(feature = "textmode-normal")]
        {
            self.ui_state.current_mode = AppMode::Edit;
            self.ui_state.selection = SelectionState::None;

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Edit);
            }
        }

        // Default (not normal): original vim behavior
        #[cfg(not(feature = "textmode-normal"))]
        match (self.ui_state.current_mode, mode) {
            (AppMode::ReadOnly, AppMode::Highlight) => {
                self.enter_highlight_mode();
            }
            (AppMode::Highlight, AppMode::ReadOnly) => {
                self.exit_highlight_mode();
            }
            (_, new_mode) => {
                self.ui_state.current_mode = new_mode;
                if new_mode != AppMode::Highlight {
                    self.ui_state.selection = SelectionState::None;
                }
                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(new_mode);
                }
            }
        }
    }

    /// Exit edit mode to read-only mode
    pub fn exit_edit_mode(&mut self) -> anyhow::Result<()> {
        #[cfg(feature = "validation")]
        {
            let current_text = self.current_text();
            if !self.ui_state.validation.allows_field_switch(
                self.ui_state.current_field,
                current_text,
            ) {
                if let Some(reason) = self
                    .ui_state
                    .validation
                    .field_switch_block_reason(
                        self.ui_state.current_field,
                        current_text,
                    )
                {
                    self.ui_state
                        .validation
                        .set_last_switch_block(reason.clone());
                    return Err(anyhow::anyhow!(
                        "Cannot exit edit mode: {}",
                        reason
                    ));
                }
            }
        }

        let current_text = self.current_text();
        if !current_text.is_empty() {
            let max_normal_pos =
                current_text.chars().count().saturating_sub(1);
            if self.ui_state.cursor_pos > max_normal_pos {
                self.ui_state.cursor_pos = max_normal_pos;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) =
                self.ui_state.validation.get_field_config(field_index)
            {
                if cfg.external_validation_enabled {
                    let text = self.current_text().to_string();
                    if !text.is_empty() {
                        self.set_external_validation(
                            field_index,
                            crate::validation::ExternalValidationState::Validating,
                        );
                        if let Some(cb) =
                            self.external_validation_callback.as_mut()
                        {
                            let final_state = cb(field_index, &text);
                            self.set_external_validation(field_index, final_state);
                        }
                    }
                }
            }
        }

        // NORMALMODE: stay in Edit (do not switch to ReadOnly)
        #[cfg(feature = "textmode-normal")]
        {
            #[cfg(feature = "suggestions")]
            {
                self.close_suggestions();
            }
            Ok(())
        }

        // Default (not normal): original vim behavior
        #[cfg(not(feature = "textmode-normal"))]
        {
            self.set_mode(AppMode::ReadOnly);
            #[cfg(feature = "suggestions")]
            {
                self.close_suggestions();
            }
            Ok(())
        }
    }

    /// Enter edit mode
    pub fn enter_edit_mode(&mut self) {
        #[cfg(feature = "computed")]
        {
            if let Some(computed_state) = &self.ui_state.computed {
                if computed_state.is_computed_field(self.ui_state.current_field)
                {
                    return;
                }
            }
        }

        // NORMALMODE: already in Edit, but enforce it
        #[cfg(feature = "textmode-normal")]
        {
            self.ui_state.current_mode = AppMode::Edit;
            self.ui_state.selection = SelectionState::None;
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Edit);
            }
        }

        // Default (not normal): vim behavior
        #[cfg(not(feature = "textmode-normal"))]
        self.set_mode(AppMode::Edit);
    }

    // -------------------- Highlight/Visual mode -------------------------

    pub fn enter_highlight_mode(&mut self) {
        // NORMALMODE: ignore request (stay in Edit)
        #[cfg(feature = "textmode-normal")]
        {
        }

        // Default (not normal): original vim
        #[cfg(not(feature = "textmode-normal"))]
        {
            if self.ui_state.current_mode == AppMode::ReadOnly {
                self.ui_state.current_mode = AppMode::Highlight;
                self.ui_state.selection = SelectionState::Characterwise {
                    anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                };

                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(AppMode::Highlight);
                }
            }
        }
    }

    pub fn enter_highlight_line_mode(&mut self) {
        // NORMALMODE: ignore
        #[cfg(feature = "textmode-normal")]
        {
        }

        // Default (not normal): original vim
        #[cfg(not(feature = "textmode-normal"))]
        {
            if self.ui_state.current_mode == AppMode::ReadOnly {
                self.ui_state.current_mode = AppMode::Highlight;
                self.ui_state.selection =
                    SelectionState::Linewise { anchor_field: self.ui_state.current_field };

                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(AppMode::Highlight);
                }
            }
        }
    }

    pub fn exit_highlight_mode(&mut self) {
        // NORMALMODE: ignore
        #[cfg(feature = "textmode-normal")]
        {
        }

        // Default (not normal): original vim
        #[cfg(not(feature = "textmode-normal"))]
        {
            if self.ui_state.current_mode == AppMode::Highlight {
                self.ui_state.current_mode = AppMode::ReadOnly;
                self.ui_state.selection = SelectionState::None;

                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(AppMode::ReadOnly);
                }
            }
        }
    }

    pub fn is_highlight_mode(&self) -> bool {
        #[cfg(feature = "textmode-normal")]
        {
            false
        }
        #[cfg(not(feature = "textmode-normal"))]
        {
            return self.ui_state.current_mode == AppMode::Highlight;
        }
    }

    pub fn selection_state(&self) -> &SelectionState {
        &self.ui_state.selection
    }

    // Visual-mode movements reuse existing movement methods
    // These keep calling the movement methods; in normalmode selection is never enabled,
    // so these just move without creating a selection.
    pub fn move_left_with_selection(&mut self) {
        let _ = self.move_left();
    }

    pub fn move_right_with_selection(&mut self) {
        let _ = self.move_right();
    }

    pub fn move_up_with_selection(&mut self) {
        let _ = self.move_up();
    }

    pub fn move_down_with_selection(&mut self) {
        let _ = self.move_down();
    }

    pub fn move_word_next_with_selection(&mut self) {
        self.move_word_next();
    }

    pub fn move_word_end_with_selection(&mut self) {
        self.move_word_end();
    }

    pub fn move_word_prev_with_selection(&mut self) {
        self.move_word_prev();
    }

    pub fn move_word_end_prev_with_selection(&mut self) {
        self.move_word_end_prev();
    }

    pub fn move_big_word_next_with_selection(&mut self) {
        self.move_big_word_next();
    }

    pub fn move_big_word_end_with_selection(&mut self) {
        self.move_big_word_end();
    }

    pub fn move_big_word_prev_with_selection(&mut self) {
        self.move_big_word_prev();
    }

    pub fn move_big_word_end_prev_with_selection(&mut self) {
        self.move_big_word_end_prev();
    }

    pub fn move_line_start_with_selection(&mut self) {
        self.move_line_start();
    }

    pub fn move_line_end_with_selection(&mut self) {
        self.move_line_end();
    }
}
