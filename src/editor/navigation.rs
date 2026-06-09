// src/editor/navigation.rs
use crate::editor::EditorCore;
use crate::DataProvider;

impl<D: DataProvider> EditorCore<D> {
    /// Resolves the target field, skipping those computed by scripts rather than human input.
    #[cfg(feature = "computed")]
    fn resolved_navigable_field(
        &self,
        requested_field: usize,
        prev_field: usize,
        field_count: usize,
    ) -> usize {
        let target_field = requested_field.min(field_count - 1);

        let Some(computed_state) = &self.ui_state.computed else {
            return target_field;
        };

        if !computed_state.is_computed_field(target_field) {
            return target_field;
        }

        let search_forward_first = target_field >= prev_field;

        let mut search_forward =
            || ((target_field + 1)..field_count).find(|&i| !computed_state.is_computed_field(i));
        let mut search_backward = || {
            (0..target_field)
                .rev()
                .find(|&i| !computed_state.is_computed_field(i))
        };

        if search_forward_first {
            search_forward()
                .or_else(&mut search_backward)
                .unwrap_or(prev_field)
        } else {
            search_backward()
                .or_else(&mut search_forward)
                .unwrap_or(prev_field)
        }
    }

    pub fn transition_to_field(&mut self, new_field: usize) -> anyhow::Result<()> {
        // Switching fields ends any in-progress undo-coalescing run.
        self.break_undo_coalescing();

        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }

        let prev_field = self.ui_state.current_field;

        #[cfg(feature = "computed")]
        let target_field = self.resolved_navigable_field(new_field, prev_field, field_count);
        #[cfg(not(feature = "computed"))]
        let target_field = new_field.min(field_count - 1);

        if target_field == prev_field {
            return Ok(());
        }

        #[cfg(feature = "validation")]
        self.ui_state.validation.clear_last_switch_block();

        #[cfg(feature = "validation")]
        {
            let current_text = self.current_text();
            if !self
                .ui_state
                .validation
                .allows_field_switch(prev_field, current_text)
            {
                if let Some(reason) = self
                    .ui_state
                    .validation
                    .field_switch_block_reason(prev_field, current_text)
                {
                    self.ui_state
                        .validation
                        .set_last_switch_block(reason.clone());
                    tracing::debug!("Field switch blocked: {}", reason);
                    return Err(anyhow::anyhow!("Cannot switch fields: {}", reason));
                }
            }
        }

        #[cfg(feature = "validation")]
        {
            let text = self.data_provider.field_value(prev_field).to_string();
            let _ = self
                .ui_state
                .validation
                .validate_field_content(prev_field, &text);

            if let Some(cfg) = self.ui_state.validation.get_field_config(prev_field) {
                if cfg.external_validation_enabled && !text.is_empty() {
                    self.set_external_validation(
                        prev_field,
                        crate::validation::ExternalValidationState::Validating,
                    );

                    if let Some(cb) = self.external_validation_callback.as_mut() {
                        let final_state = cb(prev_field, &text);
                        self.set_external_validation(prev_field, final_state);
                    }
                }
            }
        }

        let ideal_cursor_column = self.ui_state.ideal_cursor_column;
        self.ui_state.move_to_field(target_field, field_count);

        let current_text = self.current_text();
        let max_pos = current_text.chars().count();
        self.set_cursor_for_mode(ideal_cursor_column, max_pos);
        self.ui_state.ideal_cursor_column = ideal_cursor_column;

        #[cfg(feature = "suggestions")]
        {
            self.dismiss_suggestions();
        }

        Ok(())
    }

    /// Move to first line (vim gg)
    pub fn move_first_line(&mut self) -> anyhow::Result<()> {
        self.transition_to_field(0)
    }

    /// Move to last line (vim G)
    pub fn move_last_line(&mut self) -> anyhow::Result<()> {
        let last_field = self.data_provider.field_count().saturating_sub(1);
        self.transition_to_field(last_field)
    }

    /// Move to previous field (vim k / up)
    /// Returns true if moved, false if already at top
    pub fn move_up(&mut self) -> bool {
        if self.ui_state.current_field == 0 {
            return false;
        }
        let before = self.ui_state.current_field;
        let new_field = self.ui_state.current_field - 1;
        self.transition_to_field(new_field).is_ok() && self.ui_state.current_field != before
    }

    /// Move to next field (vim j / down)
    /// Returns true if moved, false if already at bottom
    pub fn move_down(&mut self) -> bool {
        let last = self.data_provider.field_count().saturating_sub(1);
        if self.ui_state.current_field >= last {
            return false;
        }
        let before = self.ui_state.current_field;
        let new_field = self.ui_state.current_field + 1;
        self.transition_to_field(new_field).is_ok() && self.ui_state.current_field != before
    }

    /// Move to next field cyclic
    pub fn move_to_next_field(&mut self) -> anyhow::Result<()> {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }
        let new_field = (self.ui_state.current_field + 1) % field_count;
        self.transition_to_field(new_field)
    }

    pub fn prev_field(&mut self) -> bool {
        self.move_up()
    }

    pub fn next_field(&mut self) -> bool {
        self.move_down()
    }
}

#[cfg(all(test, feature = "computed"))]
mod tests {
    use super::*;
    use crate::computed::{ComputedContext, ComputedProvider};

    #[derive(Clone, Default)]
    struct TestProvider {
        fields: Vec<(&'static str, String)>,
    }

    impl TestProvider {
        fn new(names: &[&'static str]) -> Self {
            Self {
                fields: names.iter().map(|name| (*name, String::new())).collect(),
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

    struct TestComputedProvider;

    impl ComputedProvider for TestComputedProvider {
        fn handles_field(&self, field_index: usize) -> bool {
            matches!(field_index, 2 | 3 | 4)
        }

        fn field_dependencies(&self, _field_index: usize) -> Vec<usize> {
            vec![0]
        }

        fn compute_field(&mut self, _context: ComputedContext) -> String {
            String::new()
        }
    }

    #[test]
    fn move_down_skips_trailing_computed_fields() {
        let provider = TestProvider::new(&["a", "b", "c", "d", "e"]);
        let mut editor = EditorCore::new(provider);
        editor.register_computed_provider(&TestComputedProvider);

        assert!(editor.transition_to_field(1).is_ok());
        assert_eq!(editor.current_field(), 1);

        assert!(!editor.move_down());
        assert_eq!(editor.current_field(), 1);
    }

    #[test]
    fn move_last_line_lands_on_last_editable_field() {
        let provider = TestProvider::new(&["a", "b", "c", "d", "e"]);
        let mut editor = EditorCore::new(provider);
        editor.register_computed_provider(&TestComputedProvider);

        assert!(editor.move_last_line().is_ok());
        assert_eq!(editor.current_field(), 1);
    }
}
