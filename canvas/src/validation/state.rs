//! Validation state management

use crate::validation::{ValidationConfig, ValidationResult};
use std::collections::HashMap;

/// Validation state for all fields in a form
#[derive(Debug, Clone, Default)]
pub struct ValidationState {
    /// Validation configurations per field index
    field_configs: HashMap<usize, ValidationConfig>,
    
    /// Current validation results per field index
    field_results: HashMap<usize, ValidationResult>,
    
    /// Track which fields have been validated
    validated_fields: std::collections::HashSet<usize>,
    
    /// Global validation enabled/disabled
    enabled: bool,
}

impl ValidationState {
    /// Create a new validation state
    pub fn new() -> Self {
        Self {
            field_configs: HashMap::new(),
            field_results: HashMap::new(),
            validated_fields: std::collections::HashSet::new(),
            enabled: true,
        }
    }
    
    /// Enable or disable validation globally
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            // Clear all validation results when disabled
            self.field_results.clear();
            self.validated_fields.clear();
        }
    }
    
    /// Check if validation is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Set validation configuration for a field
    pub fn set_field_config(&mut self, field_index: usize, config: ValidationConfig) {
        if config.has_validation() {
            self.field_configs.insert(field_index, config);
        } else {
            self.field_configs.remove(&field_index);
            self.field_results.remove(&field_index);
            self.validated_fields.remove(&field_index);
        }
    }
    
    /// Get validation configuration for a field
    pub fn get_field_config(&self, field_index: usize) -> Option<&ValidationConfig> {
        self.field_configs.get(&field_index)
    }
    
    /// Remove validation configuration for a field
    pub fn remove_field_config(&mut self, field_index: usize) {
        self.field_configs.remove(&field_index);
        self.field_results.remove(&field_index);
        self.validated_fields.remove(&field_index);
    }
    
    /// Validate character insertion for a field
    pub fn validate_char_insertion(
        &mut self,
        field_index: usize,
        current_text: &str,
        position: usize,
        character: char,
    ) -> ValidationResult {
        if !self.enabled {
            return ValidationResult::Valid;
        }
        
        if let Some(config) = self.field_configs.get(&field_index) {
            let result = config.validate_char_insertion(current_text, position, character);
            
            // Store the validation result
            self.field_results.insert(field_index, result.clone());
            self.validated_fields.insert(field_index);
            
            result
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Validate field content
    pub fn validate_field_content(
        &mut self,
        field_index: usize,
        text: &str,
    ) -> ValidationResult {
        if !self.enabled {
            return ValidationResult::Valid;
        }
        
        if let Some(config) = self.field_configs.get(&field_index) {
            let result = config.validate_content(text);
            
            // Store the validation result
            self.field_results.insert(field_index, result.clone());
            self.validated_fields.insert(field_index);
            
            result
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Get current validation result for a field
    pub fn get_field_result(&self, field_index: usize) -> Option<&ValidationResult> {
        self.field_results.get(&field_index)
    }
    
    /// Check if a field has been validated
    pub fn is_field_validated(&self, field_index: usize) -> bool {
        self.validated_fields.contains(&field_index)
    }
    
    /// Clear validation result for a field
    pub fn clear_field_result(&mut self, field_index: usize) {
        self.field_results.remove(&field_index);
        self.validated_fields.remove(&field_index);
    }
    
    /// Clear all validation results
    pub fn clear_all_results(&mut self) {
        self.field_results.clear();
        self.validated_fields.clear();
    }
    
    /// Get all field indices that have validation configured
    pub fn validated_field_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.field_configs.keys().copied()
    }
    
    /// Get all field indices with validation errors
    pub fn fields_with_errors(&self) -> impl Iterator<Item = usize> + '_ {
        self.field_results
            .iter()
            .filter(|(_, result)| result.is_error())
            .map(|(index, _)| *index)
    }
    
    /// Get all field indices with validation warnings
    pub fn fields_with_warnings(&self) -> impl Iterator<Item = usize> + '_ {
        self.field_results
            .iter()
            .filter(|(_, result)| matches!(result, ValidationResult::Warning { .. }))
            .map(|(index, _)| *index)
    }
    
    /// Check if any field has validation errors
    pub fn has_errors(&self) -> bool {
        self.field_results.values().any(|result| result.is_error())
    }
    
    /// Check if any field has validation warnings
    pub fn has_warnings(&self) -> bool {
        self.field_results.values().any(|result| matches!(result, ValidationResult::Warning { .. }))
    }
    
    /// Get total count of fields with validation configured
    pub fn validated_field_count(&self) -> usize {
        self.field_configs.len()
    }
    
    /// Get validation summary
    pub fn summary(&self) -> ValidationSummary {
        let total_validated = self.validated_fields.len();
        let errors = self.fields_with_errors().count();
        let warnings = self.fields_with_warnings().count();
        let valid = total_validated - errors - warnings;
        
        ValidationSummary {
            total_fields: self.field_configs.len(),
            validated_fields: total_validated,
            valid_fields: valid,
            warning_fields: warnings,
            error_fields: errors,
        }
    }
}

/// Summary of validation state across all fields
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    /// Total number of fields with validation configured
    pub total_fields: usize,
    
    /// Number of fields that have been validated
    pub validated_fields: usize,
    
    /// Number of fields with valid validation results
    pub valid_fields: usize,
    
    /// Number of fields with warnings
    pub warning_fields: usize,
    
    /// Number of fields with errors
    pub error_fields: usize,
}

impl ValidationSummary {
    /// Check if all configured fields are valid
    pub fn is_all_valid(&self) -> bool {
        self.error_fields == 0 && self.validated_fields == self.total_fields
    }
    
    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.error_fields > 0
    }
    
    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.warning_fields > 0
    }
    
    /// Get completion percentage (validated fields / total fields)
    pub fn completion_percentage(&self) -> f32 {
        if self.total_fields == 0 {
            1.0
        } else {
            self.validated_fields as f32 / self.total_fields as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{CharacterLimits, ValidationConfigBuilder};

    #[test]
    fn test_validation_state_creation() {
        let state = ValidationState::new();
        assert!(state.is_enabled());
        assert_eq!(state.validated_field_count(), 0);
    }
    
    #[test]
    fn test_enable_disable() {
        let mut state = ValidationState::new();
        
        // Add some validation config
        let config = ValidationConfigBuilder::new()
            .with_max_length(10)
            .build();
        state.set_field_config(0, config);
        
        // Validate something
        let result = state.validate_field_content(0, "test");
        assert!(result.is_acceptable());
        assert!(state.is_field_validated(0));
        
        // Disable validation
        state.set_enabled(false);
        assert!(!state.is_enabled());
        assert!(!state.is_field_validated(0)); // Should be cleared
        
        // Validation should now return valid regardless
        let result = state.validate_field_content(0, "this is way too long for the limit");
        assert!(result.is_acceptable());
    }
    
    #[test]
    fn test_field_config_management() {
        let mut state = ValidationState::new();
        
        let config = ValidationConfigBuilder::new()
            .with_max_length(5)
            .build();
        
        // Set config
        state.set_field_config(0, config);
        assert_eq!(state.validated_field_count(), 1);
        assert!(state.get_field_config(0).is_some());
        
        // Remove config
        state.remove_field_config(0);
        assert_eq!(state.validated_field_count(), 0);
        assert!(state.get_field_config(0).is_none());
    }
    
    #[test]
    fn test_character_insertion_validation() {
        let mut state = ValidationState::new();
        
        let config = ValidationConfigBuilder::new()
            .with_max_length(5)
            .build();
        state.set_field_config(0, config);
        
        // Valid insertion
        let result = state.validate_char_insertion(0, "test", 4, 'x');
        assert!(result.is_acceptable());
        
        // Invalid insertion
        let result = state.validate_char_insertion(0, "tests", 5, 'x');
        assert!(!result.is_acceptable());
        
        // Check that result was stored
        assert!(state.is_field_validated(0));
        let stored_result = state.get_field_result(0);
        assert!(stored_result.is_some());
        assert!(!stored_result.unwrap().is_acceptable());
    }
    
    #[test]
    fn test_validation_summary() {
        let mut state = ValidationState::new();
        
        // Configure two fields
        let config1 = ValidationConfigBuilder::new().with_max_length(5).build();
        let config2 = ValidationConfigBuilder::new().with_max_length(10).build();
        state.set_field_config(0, config1);
        state.set_field_config(1, config2);
        
        // Validate field 0 (valid)
        state.validate_field_content(0, "test");
        
        // Validate field 1 (error)
        state.validate_field_content(1, "this is too long");
        
        let summary = state.summary();
        assert_eq!(summary.total_fields, 2);
        assert_eq!(summary.validated_fields, 2);
        assert_eq!(summary.valid_fields, 1);
        assert_eq!(summary.error_fields, 1);
        assert_eq!(summary.warning_fields, 0);
        
        assert!(!summary.is_all_valid());
        assert!(summary.has_errors());
        assert!(!summary.has_warnings());
        assert_eq!(summary.completion_percentage(), 1.0);
    }
    
    #[test]
    fn test_error_and_warning_tracking() {
        let mut state = ValidationState::new();
        
        let config = ValidationConfigBuilder::new()
            .with_character_limits(
                CharacterLimits::new_range(3, 10).with_warning_threshold(8)
            )
            .build();
        state.set_field_config(0, config);
        
        // Too short (warning)
        state.validate_field_content(0, "hi");
        assert!(state.has_warnings());
        assert!(!state.has_errors());
        
        // Just right
        state.validate_field_content(0, "hello");
        assert!(!state.has_warnings());
        assert!(!state.has_errors());
        
        // Too long (error)
        state.validate_field_content(0, "hello world!");
        assert!(!state.has_warnings());
        assert!(state.has_errors());
    }
}
