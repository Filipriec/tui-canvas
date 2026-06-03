// src/editor/mode.rs

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    fn set_highlight_mode_selection(&mut self, selection: SelectionState) {
        self.ui_state.current_mode = AppMode::Highlight;
        self.ui_state.selection = selection;

        #[cfg(feature = "cursor-style")]
        {
            let _ = CursorManager::update_for_mode(AppMode::Highlight);
        }
    }

    /// Change mode
    pub fn set_mode(&mut self, mode: AppMode) {
        // A genuine mode change ends any in-progress undo-coalescing run. In
        // normal mode the mode never actually changes (always Edit), and the
        // wrappers call `enter_edit_mode` on every keystroke, so we must only
        // break on a real transition to keep typing coalesced.
        #[cfg(not(feature = "textmode-normal"))]
        if self.ui_state.current_mode != mode {
            self.break_undo_coalescing();
        }

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
            if !self
                .ui_state
                .validation
                .allows_field_switch(self.ui_state.current_field, current_text)
            {
                if let Some(reason) = self
                    .ui_state
                    .validation
                    .field_switch_block_reason(self.ui_state.current_field, current_text)
                {
                    self.ui_state
                        .validation
                        .set_last_switch_block(reason.clone());
                    return Err(anyhow::anyhow!("Cannot exit edit mode: {}", reason));
                }
            }
        }

        let current_text = self.current_text();
        if !current_text.is_empty() {
            let max_normal_pos = current_text.chars().count().saturating_sub(1);
            if self.ui_state.cursor_pos > max_normal_pos {
                self.set_cursor_raw(max_normal_pos);
            }
        }

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if cfg.external_validation_enabled {
                    let text = self.current_text().to_string();
                    if !text.is_empty() {
                        self.set_external_validation(
                            field_index,
                            crate::validation::ExternalValidationState::Validating,
                        );
                        if let Some(cb) = self.external_validation_callback.as_mut() {
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
                self.dismiss_suggestions();
            }
            Ok(())
        }

        // Default (not normal): original vim behavior
        #[cfg(not(feature = "textmode-normal"))]
        {
            self.set_mode(AppMode::ReadOnly);
            #[cfg(feature = "suggestions")]
            {
                self.dismiss_suggestions();
            }
            Ok(())
        }
    }

    /// Enter edit mode
    pub fn enter_edit_mode(&mut self) {
        #[cfg(feature = "computed")]
        {
            if let Some(computed_state) = &self.ui_state.computed {
                if computed_state.is_computed_field(self.ui_state.current_field) {
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

        // Check if suggestions should be shown based on trigger
        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();
    }

    // Highlight/Visual mode

    pub fn enter_highlight_mode(&mut self) {
        // NORMALMODE: ignore request (stay in Edit)
        #[cfg(feature = "textmode-normal")]
        {}

        // Default (not normal): original vim
        #[cfg(not(feature = "textmode-normal"))]
        {
            match (&self.ui_state.current_mode, &self.ui_state.selection) {
                (AppMode::ReadOnly, _) => {
                    self.set_highlight_mode_selection(SelectionState::Characterwise {
                        anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                    });
                }
                (AppMode::Highlight, SelectionState::Characterwise { .. }) => {
                    self.exit_highlight_mode();
                }
                (AppMode::Highlight, _) => {
                    self.set_highlight_mode_selection(SelectionState::Characterwise {
                        anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                    });
                }
                _ => {}
            }
        }
    }

    pub fn enter_highlight_line_mode(&mut self) {
        // NORMALMODE: ignore
        #[cfg(feature = "textmode-normal")]
        {}

        // Default (not normal): original vim
        #[cfg(not(feature = "textmode-normal"))]
        {
            match (&self.ui_state.current_mode, &self.ui_state.selection) {
                (AppMode::ReadOnly, _) => {
                    self.set_highlight_mode_selection(SelectionState::Linewise {
                        anchor_field: self.ui_state.current_field,
                    });
                }
                (AppMode::Highlight, SelectionState::Linewise { .. }) => {
                    self.exit_highlight_mode();
                }
                (AppMode::Highlight, _) => {
                    self.set_highlight_mode_selection(SelectionState::Linewise {
                        anchor_field: self.ui_state.current_field,
                    });
                }
                _ => {}
            }
        }
    }

    pub fn exit_highlight_mode(&mut self) {
        // NORMALMODE: ignore
        #[cfg(feature = "textmode-normal")]
        {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::modes::AppMode;

    #[derive(Clone)]
    struct TestProvider {
        fields: Vec<(&'static str, String)>,
    }

    impl TestProvider {
        fn new(values: &[&'static str]) -> Self {
            Self {
                fields: values
                    .iter()
                    .enumerate()
                    .map(|(i, value)| {
                        let name = match i {
                            0 => "a",
                            1 => "b",
                            _ => "c",
                        };
                        (name, (*value).to_string())
                    })
                    .collect(),
            }
        }
    }

    impl DataProvider for TestProvider {
        fn field_count(&self) -> usize {
            self.fields.len()
        }

        fn field_name(&self, index: usize) -> &str {
            self.fields[index].0
        }

        fn field_value(&self, index: usize) -> &str {
            &self.fields[index].1
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            self.fields[index].1 = value;
        }
    }

    #[test]
    fn visual_characterwise_toggles_and_switches_from_linewise() {
        let mut editor = FormEditor::new(TestProvider::new(&["alpha", "beta"]));

        editor.enter_highlight_mode();
        assert_eq!(editor.mode(), AppMode::Highlight);
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        editor.enter_highlight_mode();
        assert_eq!(editor.mode(), AppMode::ReadOnly);
        assert!(matches!(editor.selection_state(), SelectionState::None));

        editor.enter_highlight_line_mode();
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        editor.move_down();
        editor.enter_highlight_mode();
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Characterwise { anchor: (1, 0) }
        ));
    }

    #[test]
    fn visual_linewise_toggles_and_switches_from_characterwise() {
        let mut editor = FormEditor::new(TestProvider::new(&["alpha", "beta"]));

        editor.enter_highlight_line_mode();
        assert_eq!(editor.mode(), AppMode::Highlight);
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        editor.enter_highlight_line_mode();
        assert_eq!(editor.mode(), AppMode::ReadOnly);
        assert!(matches!(editor.selection_state(), SelectionState::None));

        editor.enter_highlight_mode();
        editor.move_down();
        editor.enter_highlight_line_mode();
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Linewise { anchor_field: 1 }
        ));
    }
}
