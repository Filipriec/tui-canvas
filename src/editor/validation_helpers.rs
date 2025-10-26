// src/editor/validation_helpers.rs

use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    #[cfg(feature = "validation")]
    pub fn set_validation_enabled(&mut self, enabled: bool) {
        self.ui_state.validation.set_enabled(enabled);
    }

    #[cfg(feature = "validation")]
    pub fn is_validation_enabled(&self) -> bool {
        self.ui_state.validation.is_enabled()
    }

    #[cfg(feature = "validation")]
    pub fn set_field_validation(
        &mut self,
        field_index: usize,
        config: crate::validation::ValidationConfig,
    ) {
        self.ui_state
            .validation
            .set_field_config(field_index, config);
    }

    #[cfg(feature = "validation")]
    pub fn remove_field_validation(&mut self, field_index: usize) {
        self.ui_state.validation.remove_field_config(field_index);
    }

    #[cfg(feature = "validation")]
    pub fn validate_current_field(
        &mut self,
    ) -> crate::validation::ValidationResult {
        let field_index = self.ui_state.current_field;
        let current_text = self.current_text().to_string();
        self.ui_state
            .validation
            .validate_field_content(field_index, &current_text)
    }

    #[cfg(feature = "validation")]
    pub fn validate_field(
        &mut self,
        field_index: usize,
    ) -> Option<crate::validation::ValidationResult> {
        if field_index < self.data_provider.field_count() {
            let text =
                self.data_provider.field_value(field_index).to_string();
            Some(
                self.ui_state
                    .validation
                    .validate_field_content(field_index, &text),
            )
        } else {
            None
        }
    }

    #[cfg(feature = "validation")]
    pub fn clear_validation_results(&mut self) {
        self.ui_state.validation.clear_all_results();
    }

    #[cfg(feature = "validation")]
    pub fn validation_summary(
        &self,
    ) -> crate::validation::ValidationSummary {
        self.ui_state.validation.summary()
    }

    #[cfg(feature = "validation")]
    pub fn can_switch_fields(&self) -> bool {
        let current_text = self.current_text();
        self.ui_state.validation.allows_field_switch(
            self.ui_state.current_field,
            current_text,
        )
    }

    #[cfg(feature = "validation")]
    pub fn field_switch_block_reason(&self) -> Option<String> {
        let current_text = self.current_text();
        self.ui_state.validation.field_switch_block_reason(
            self.ui_state.current_field,
            current_text,
        )
    }

    #[cfg(feature = "validation")]
    pub fn last_switch_block(&self) -> Option<&str> {
        self.ui_state.validation.last_switch_block()
    }

    #[cfg(feature = "validation")]
    pub fn current_limits_status_text(&self) -> Option<String> {
        let idx = self.ui_state.current_field;
        if let Some(cfg) = self.ui_state.validation.get_field_config(idx) {
            if let Some(limits) = &cfg.character_limits {
                return limits.status_text(self.current_text());
            }
        }
        None
    }

    #[cfg(feature = "validation")]
    pub fn current_formatter_warning(&self) -> Option<String> {
        let idx = self.ui_state.current_field;
        if let Some(cfg) = self.ui_state.validation.get_field_config(idx) {
            if let Some((_fmt, _mapper, warn)) =
                cfg.run_custom_formatter(self.current_text())
            {
                return warn;
            }
        }
        None
    }

    #[cfg(feature = "validation")]
    pub fn external_validation_of(
        &self,
        field_index: usize,
    ) -> crate::validation::ExternalValidationState {
        self.ui_state
            .validation
            .get_external_validation(field_index)
    }

    #[cfg(feature = "validation")]
    pub fn clear_all_external_validation(&mut self) {
        self.ui_state.validation.clear_all_external_validation();
    }

    #[cfg(feature = "validation")]
    pub fn clear_external_validation(&mut self, field_index: usize) {
        self.ui_state
            .validation
            .clear_external_validation(field_index);
    }

    #[cfg(feature = "validation")]
    pub fn set_external_validation(
        &mut self,
        field_index: usize,
        state: crate::validation::ExternalValidationState,
    ) {
        self.ui_state
            .validation
            .set_external_validation(field_index, state);
    }

    #[cfg(feature = "validation")]
    pub fn set_external_validation_callback<F>(&mut self, callback: F)
    where
        F: FnMut(usize, &str) -> crate::validation::ExternalValidationState
            + Send
            + Sync
            + 'static,
    {
        self.external_validation_callback = Some(Box::new(callback));
    }

    #[cfg(feature = "validation")]
    pub(crate) fn initialize_validation(&mut self) {
        let field_count = self.data_provider.field_count();
        for field_index in 0..field_count {
            if let Some(config) =
                self.data_provider.validation_config(field_index)
            {
                self.ui_state
                    .validation
                    .set_field_config(field_index, config);
            }
        }
    }
}
