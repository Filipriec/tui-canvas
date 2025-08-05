// src/validation/config.rs
//! Validation configuration types and builders

use crate::validation::{CharacterLimits, PatternFilters};

/// Main validation configuration for a field
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    /// Character limit configuration
    pub character_limits: Option<CharacterLimits>,
    
    /// Pattern filtering configuration
    pub pattern_filters: Option<PatternFilters>,
    
    /// Future: Reserved characters
    pub reserved_chars: Option<()>, // Placeholder for future implementation
    
    /// Future: Custom formatting
    pub custom_formatting: Option<()>, // Placeholder for future implementation
    
    /// Future: External validation
    pub external_validation: Option<()>, // Placeholder for future implementation
}

/// Builder for creating validation configurations
#[derive(Debug, Default)]
pub struct ValidationConfigBuilder {
    config: ValidationConfig,
}

impl ValidationConfigBuilder {
    /// Create a new validation config builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set character limits for the field
    pub fn with_character_limits(mut self, limits: CharacterLimits) -> Self {
        self.config.character_limits = Some(limits);
        self
    }
    
    /// Set pattern filters for the field
    pub fn with_pattern_filters(mut self, filters: PatternFilters) -> Self {
        self.config.pattern_filters = Some(filters);
        self
    }
    
    /// Set maximum number of characters (convenience method)
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.config.character_limits = Some(CharacterLimits::new(max_length));
        self
    }
    
    /// Build the final validation configuration
    pub fn build(self) -> ValidationConfig {
        self.config
    }
}

/// Result of a validation operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Validation passed
    Valid,
    
    /// Validation failed with warning (input still accepted)
    Warning { message: String },
    
    /// Validation failed with error (input rejected)
    Error { message: String },
}

impl ValidationResult {
    /// Check if the validation result allows the input
    pub fn is_acceptable(&self) -> bool {
        matches!(self, ValidationResult::Valid | ValidationResult::Warning { .. })
    }
    
    /// Check if the validation result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, ValidationResult::Error { .. })
    }
    
    /// Get the message if there is one
    pub fn message(&self) -> Option<&str> {
        match self {
            ValidationResult::Valid => None,
            ValidationResult::Warning { message } => Some(message),
            ValidationResult::Error { message } => Some(message),
        }
    }
    
    /// Create a warning result
    pub fn warning(message: impl Into<String>) -> Self {
        ValidationResult::Warning { message: message.into() }
    }
    
    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        ValidationResult::Error { message: message.into() }
    }
}

impl ValidationConfig {
    /// Create a new empty validation configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a configuration with just character limits
    pub fn with_max_length(max_length: usize) -> Self {
        ValidationConfigBuilder::new()
            .with_max_length(max_length)
            .build()
    }
    
    /// Create a configuration with pattern filters
    pub fn with_patterns(patterns: PatternFilters) -> Self {
        ValidationConfigBuilder::new()
            .with_pattern_filters(patterns)
            .build()
    }
    
    /// Validate a character insertion at a specific position
    pub fn validate_char_insertion(
        &self,
        current_text: &str,
        position: usize,
        character: char,
    ) -> ValidationResult {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            if let Some(result) = limits.validate_insertion(current_text, position, character) {
                if !result.is_acceptable() {
                    return result;
                }
            }
        }
        
        // Pattern filters validation
        if let Some(ref patterns) = self.pattern_filters {
            if let Err(message) = patterns.validate_char_at_position(position, character) {
                return ValidationResult::error(message);
            }
        }
        
        // Future: Add other validation types here
        
        ValidationResult::Valid
    }
    
    /// Validate the current text content
    pub fn validate_content(&self, text: &str) -> ValidationResult {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            if let Some(result) = limits.validate_content(text) {
                if !result.is_acceptable() {
                    return result;
                }
            }
        }
        
        // Pattern filters validation
        if let Some(ref patterns) = self.pattern_filters {
            if let Err(message) = patterns.validate_text(text) {
                return ValidationResult::error(message);
            }
        }
        
        // Future: Add other validation types here
        
        ValidationResult::Valid
    }
    
    /// Check if any validation rules are configured
    pub fn has_validation(&self) -> bool {
        self.character_limits.is_some() || self.pattern_filters.is_some()
        // || self.reserved_chars.is_some()
        // || self.custom_formatting.is_some()
        // || self.external_validation.is_some()
    }
    pub fn allows_field_switch(&self, text: &str) -> bool {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            if !limits.allows_field_switch(text) {
                return false;
            }
        }
        
        // Future: Add other validation types here
        
        true
    }
    
    /// Get reason why field switching is blocked (if any)
    pub fn field_switch_block_reason(&self, text: &str) -> Option<String> {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            if let Some(reason) = limits.field_switch_block_reason(text) {
                return Some(reason);
            }
        }
        
        // Future: Add other validation types here
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_config_builder() {
        let config = ValidationConfigBuilder::new()
            .with_max_length(10)
            .build();
        
        assert!(config.character_limits.is_some());
        assert_eq!(config.character_limits.unwrap().max_length(), Some(10));
    }
    
    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::Valid;
        assert!(valid.is_acceptable());
        assert!(!valid.is_error());
        assert_eq!(valid.message(), None);
        
        let warning = ValidationResult::warning("Too long");
        assert!(warning.is_acceptable());
        assert!(!warning.is_error());
        assert_eq!(warning.message(), Some("Too long"));
        
        let error = ValidationResult::error("Invalid");
        assert!(!error.is_acceptable());
        assert!(error.is_error());
        assert_eq!(error.message(), Some("Invalid"));
    }
    
    #[test]
    fn test_config_with_max_length() {
        let config = ValidationConfig::with_max_length(5);
        assert!(config.has_validation());
        
        // Test valid insertion
        let result = config.validate_char_insertion("test", 4, 'x');
        assert!(result.is_acceptable());
        
        // Test invalid insertion (would exceed limit)
        let result = config.validate_char_insertion("tests", 5, 'x');
        assert!(!result.is_acceptable());
    }
    
    #[test]
    fn test_config_with_patterns() {
        use crate::validation::{PatternFilters, PositionFilter, PositionRange, CharacterFilter};
        
        let patterns = PatternFilters::new()
            .add_filter(PositionFilter::new(
                PositionRange::Range(0, 1),
                CharacterFilter::Alphabetic,
            ));
        
        let config = ValidationConfig::with_patterns(patterns);
        assert!(config.has_validation());
        
        // Test valid pattern insertion
        let result = config.validate_char_insertion("", 0, 'A');
        assert!(result.is_acceptable());
        
        // Test invalid pattern insertion  
        let result = config.validate_char_insertion("", 0, '1');
        assert!(!result.is_acceptable());
    }
    
    #[test]
    fn test_config_builder_with_patterns() {
        use crate::validation::{PatternFilters, PositionFilter, PositionRange, CharacterFilter};
        
        let patterns = PatternFilters::license_plate();
        let config = ValidationConfigBuilder::new()
            .with_pattern_filters(patterns)
            .with_max_length(5)
            .build();
        
        assert!(config.has_validation());
        assert!(config.character_limits.is_some());
        assert!(config.pattern_filters.is_some());
        
        // Test pattern validation
        let result = config.validate_content("AB123");
        assert!(result.is_acceptable());
        
        let result = config.validate_content("A1123");
        assert!(!result.is_acceptable());
    }
}
