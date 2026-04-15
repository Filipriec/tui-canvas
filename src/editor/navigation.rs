// src/editor/navigation.rs
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    pub fn transition_to_field(&mut self, new_field: usize) -> anyhow::Result<()> {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }

        let prev_field = self.ui_state.current_field;

        #[cfg(feature = "computed")]
        let mut target_field = new_field.min(field_count - 1);
        #[cfg(not(feature = "computed"))]
        let target_field = new_field.min(field_count - 1);

        #[cfg(feature = "computed")]
        {
            if let Some(computed_state) = &self.ui_state.computed {
                if computed_state.is_computed_field(target_field) {
                    if target_field >= prev_field {
                        for i in (target_field + 1)..field_count {
                            if !computed_state.is_computed_field(i) {
                                target_field = i;
                                break;
                            }
                        }
                    } else {
                        let mut i = target_field;
                        loop {
                            if !computed_state.is_computed_field(i) {
                                target_field = i;
                                break;
                            }
                            if i == 0 {
                                break;
                            }
                            i -= 1;
                        }
                    }
                }
            }
        }

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
                    return Err(anyhow::anyhow!(
                        "Cannot switch fields: {}",
                        reason
                    ));
                }
            }
        }

        #[cfg(feature = "validation")]
        {
            let text =
                self.data_provider.field_value(prev_field).to_string();
            let _ = self
                .ui_state
                .validation
                .validate_field_content(prev_field, &text);

            if let Some(cfg) =
                self.ui_state.validation.get_field_config(prev_field)
            {
                if cfg.external_validation_enabled && !text.is_empty() {
                    self.set_external_validation(
                        prev_field,
                        crate::validation::ExternalValidationState::Validating,
                    );

                    if let Some(cb) =
                        self.external_validation_callback.as_mut()
                    {
                        let final_state = cb(prev_field, &text);
                        self.set_external_validation(prev_field, final_state);
                    }
                }
            }
        }

        self.ui_state.move_to_field(target_field, field_count);

        let current_text = self.current_text();
        let max_pos = current_text.chars().count();
        self.set_cursor_for_mode(self.ui_state.ideal_cursor_column, max_pos);

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
        let last_field =
            self.data_provider.field_count().saturating_sub(1);
        self.transition_to_field(last_field)
    }

    /// Move to previous field (vim k / up)
    /// Returns true if moved, false if already at top
    pub fn move_up(&mut self) -> bool {
        if self.ui_state.current_field == 0 {
            return false;
        }
        let new_field = self.ui_state.current_field - 1;
        self.transition_to_field(new_field).is_ok()
    }

    /// Move to next field (vim j / down)
    /// Returns true if moved, false if already at bottom
    pub fn move_down(&mut self) -> bool {
        let last = self.data_provider.field_count().saturating_sub(1);
        if self.ui_state.current_field >= last {
            return false;
        }
        let new_field = self.ui_state.current_field + 1;
        self.transition_to_field(new_field).is_ok()
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
