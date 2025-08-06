// src/validation/config.rs
//! Validation configuration types and builders

use crate::validation::{CharacterLimits, PatternFilters, DisplayMask};
#[cfg(feature = "validation")]
use crate::validation::{CustomFormatter, FormattingResult, PositionMapper};
use std::sync::Arc;

/// Main validation configuration for a field
#[derive(Clone, Default)]
pub struct ValidationConfig {
    /// Character limit configuration
    pub character_limits: Option<CharacterLimits>,

    /// Pattern filtering configuration
    pub pattern_filters: Option<PatternFilters>,

    /// User-defined display mask for visual formatting
    pub display_mask: Option<DisplayMask>,

    /// Optional: user-provided custom formatter (feature 4)
    #[cfg(feature = "validation")]
    pub custom_formatter: Option<Arc<dyn CustomFormatter + Send + Sync>>,

    /// Future: External validation
    pub external_validation: Option<()>, // Placeholder for future implementation
}

/// Manual Debug to avoid requiring Debug on dyn CustomFormatter
impl std::fmt::Debug for ValidationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("ValidationConfig");
        ds.field("character_limits", &self.character_limits)
          .field("pattern_filters", &self.pattern_filters)
          .field("display_mask", &self.display_mask)
          // Do not print the formatter itself to avoid requiring Debug
          .field(
              "custom_formatter",
              &{
                  #[cfg(feature = "validation")]
                  {
                      if self.custom_formatter.is_some() { &"Some(<CustomFormatter>)" } else { &"None" }
                  }
                  #[cfg(not(feature = "validation"))]
                  {
                      &"N/A"
                  }
              },
          )
          .field("external_validation", &self.external_validation)
          .finish()
    }
}

// ✅ FIXED: Move function from struct definition to impl block
impl ValidationConfig {
    /// If a custom formatter is configured, run it and return the formatted text,
    /// the position mapper and an optional warning message.
    ///
    /// Returns None when no custom formatter is configured.
    #[cfg(feature = "validation")]
    pub fn run_custom_formatter(
        &self,
        raw: &str,
    ) -> Option<(String, Arc<dyn PositionMapper>, Option<String>)> {
        let formatter = self.custom_formatter.as_ref()?;
        match formatter.format(raw) {
            FormattingResult::Success { formatted, mapper } => {
                Some((formatted, mapper, None))
            }
            FormattingResult::Warning { formatted, message, mapper } => {
                Some((formatted, mapper, Some(message)))
            }
            FormattingResult::Error { .. } => None, // Fall back to raw display
        }
    }

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

    /// Create a configuration with user-defined display mask
    ///
    /// # Examples
    /// ```
    /// use canvas::{ValidationConfig, DisplayMask};
    ///
    /// let phone_mask = DisplayMask::new("(###) ###-####", '#');
    /// let config = ValidationConfig::with_mask(phone_mask);
    /// ```
    pub fn with_mask(mask: DisplayMask) -> Self {
        ValidationConfigBuilder::new()
            .with_display_mask(mask)
            .build()
    }

    /// Validate a character insertion at a specific position (raw text space).
    ///
    /// Note: Display masks are visual-only and do not participate in validation.
    /// Editor logic is responsible for skipping mask separator positions; here we
    /// only validate the raw insertion against limits and patterns.
    pub fn validate_char_insertion(
        &self,
        current_text: &str,
        position: usize,
        character: char,
    ) -> ValidationResult {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            // ✅ FIXED: Explicit return type annotation
            if let Some(result) = limits.validate_insertion(current_text, position, character) {
                if !result.is_acceptable() {
                    return result;
                }
            }
        }

        // Pattern filters validation
        if let Some(ref patterns) = self.pattern_filters {
            // ✅ FIXED: Explicit error handling
            if let Err(message) = patterns.validate_char_at_position(position, character) {
                return ValidationResult::error(message);
            }
        }

        // Future: Add other validation types here

        ValidationResult::Valid
    }

    /// Validate the current text content (raw text space)
    pub fn validate_content(&self, text: &str) -> ValidationResult {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            // ✅ FIXED: Explicit return type annotation
            if let Some(result) = limits.validate_content(text) {
                if !result.is_acceptable() {
                    return result;
                }
            }
        }

        // Pattern filters validation
        if let Some(ref patterns) = self.pattern_filters {
            // ✅ FIXED: Explicit error handling
            if let Err(message) = patterns.validate_text(text) {
                return ValidationResult::error(message);
            }
        }

        // Future: Add other validation types here

        ValidationResult::Valid
    }

    /// Check if any validation rules are configured
    pub fn has_validation(&self) -> bool {
        self.character_limits.is_some()
            || self.pattern_filters.is_some()
            || self.display_mask.is_some()
            || {
                #[cfg(feature = "validation")]
                { self.custom_formatter.is_some() }
                #[cfg(not(feature = "validation"))]
                { false }
            }
    }

    pub fn allows_field_switch(&self, text: &str) -> bool {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            // ✅ FIXED: Direct boolean return
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
            // ✅ FIXED: Direct option return
            if let Some(reason) = limits.field_switch_block_reason(text) {
                return Some(reason);
            }
        }

        // Future: Add other validation types here

        None
    }
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

    /// Set user-defined display mask for visual formatting
    ///
    /// # Examples
    /// ```
    /// use canvas::{ValidationConfigBuilder, DisplayMask};
    ///
    /// // Phone number with dynamic formatting
    /// let phone_mask = DisplayMask::new("(###) ###-####", '#');
    /// let config = ValidationConfigBuilder::new()
    ///     .with_display_mask(phone_mask)
    ///     .build();
    ///
    /// // Date with template formatting
    /// let date_mask = DisplayMask::new("##/##/####", '#')
    ///     .with_template('_');
    /// let config = ValidationConfigBuilder::new()
    ///     .with_display_mask(date_mask)
    ///     .build();
    ///
    /// // Custom business format
    /// let employee_id = DisplayMask::new("EMP-####-##", '#')
    ///     .with_template('•');
    /// let config = ValidationConfigBuilder::new()
    ///     .with_display_mask(employee_id)
    ///     .with_max_length(6)  // Only store the 6 digits
    ///     .build();
    /// ```
    pub fn with_display_mask(mut self, mask: DisplayMask) -> Self {
        self.config.display_mask = Some(mask);
        self
    }

    /// Set optional custom formatter (feature 4)
    #[cfg(feature = "validation")]
    pub fn with_custom_formatter<F>(mut self, formatter: Arc<F>) -> Self
    where
        F: CustomFormatter + Send + Sync + 'static,
    {
        self.config.custom_formatter = Some(formatter);
        // When custom formatter is present, it takes precedence over display mask.
        self.config.display_mask = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_with_user_defined_mask() {
        // User creates their own phone mask
        let phone_mask = DisplayMask::new("(###) ###-####", '#');
        let config = ValidationConfig::with_mask(phone_mask);

        // has_validation should be true because mask is configured
        assert!(config.has_validation());

        // Display mask is visual only; validation still focuses on raw content
        let result = config.validate_char_insertion("123", 3, '4');
        assert!(result.is_acceptable());

        // Content validation unaffected by mask
        let result = config.validate_content("1234567890");
        assert!(result.is_acceptable());
    }

    #[test]
    fn test_validation_config_builder() {
        let config = ValidationConfigBuilder::new()
            .with_max_length(10)
            .build();

        assert!(config.character_limits.is_some());
        assert_eq!(config.character_limits.unwrap().max_length(), Some(10));
    }

    #[test]
    fn test_config_builder_with_user_mask() {
        // User defines custom format
        let custom_mask = DisplayMask::new("##-##-##", '#').with_template('_');
        let config = ValidationConfigBuilder::new()
            .with_display_mask(custom_mask)
            .with_max_length(6)
            .build();

        assert!(config.has_validation());
        assert!(config.character_limits.is_some());
        assert!(config.display_mask.is_some());
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
}
