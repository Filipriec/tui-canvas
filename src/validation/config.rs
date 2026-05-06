// src/validation/config.rs
//! Validation configuration types and builders

use crate::validation::{CharacterLimits, DisplayMask, PatternFilters};
#[cfg(feature = "validation")]
use crate::validation::{CustomFormatter, FormattingResult, PositionMapper};
use std::sync::Arc;
pub use validation_core::ValidationResult;

/// Whitelist of allowed exact values for a field.
/// If configured, the field is valid when it is empty (by default) or when the
/// content exactly matches one of the allowed values. This does not block field
/// switching (unlike minimum length in CharacterLimits).
#[derive(Clone, Debug)]
pub struct AllowedValues {
    allowed: Vec<String>,
    allow_empty: bool,
    case_insensitive: bool,
}

impl AllowedValues {
    pub fn new(allowed: Vec<String>) -> Self {
        Self {
            allowed,
            allow_empty: true,
            case_insensitive: false,
        }
    }

    /// Allow or disallow empty value to be considered valid (default: true).
    pub fn allow_empty(mut self, allow: bool) -> Self {
        self.allow_empty = allow;
        self
    }

    /// Enable/disable ASCII case-insensitive matching (default: false).
    pub fn case_insensitive(mut self, ci: bool) -> Self {
        self.case_insensitive = ci;
        self
    }

    fn matches(&self, text: &str) -> bool {
        if self.case_insensitive {
            self.allowed.iter().any(|s| s.eq_ignore_ascii_case(text))
        } else {
            self.allowed.iter().any(|s| s == text)
        }
    }
}

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

    /// Optional: restrict the field to one of exact allowed values (or empty)
    pub allowed_values: Option<AllowedValues>,

    /// Enable external validation indicator UI (feature 5)
    pub external_validation_enabled: bool,

    pub external_validation: Option<()>,
}

/// Manual Debug to avoid requiring Debug on dyn CustomFormatter
impl std::fmt::Debug for ValidationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("ValidationConfig");
        ds.field("character_limits", &self.character_limits)
            .field("pattern_filters", &self.pattern_filters)
            .field("display_mask", &self.display_mask)
            // Do not print the formatter itself to avoid requiring Debug
            .field("custom_formatter", &{
                #[cfg(feature = "validation")]
                {
                    if self.custom_formatter.is_some() {
                        &"Some(<CustomFormatter>)"
                    } else {
                        &"None"
                    }
                }
                #[cfg(not(feature = "validation"))]
                {
                    &"N/A"
                }
            })
            .field("allowed_values", &self.allowed_values)
            .field(
                "external_validation_enabled",
                &self.external_validation_enabled,
            )
            .field("external_validation", &self.external_validation)
            .finish()
    }
}

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
            FormattingResult::Success { formatted, mapper } => Some((formatted, mapper, None)),
            FormattingResult::Warning {
                formatted,
                message,
                mapper,
            } => Some((formatted, mapper, Some(message))),
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

        ValidationResult::Valid
    }

    /// Validate the current text content (raw text space)
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

        // Allowed values (whitelist) validation
        if let Some(ref allowed) = self.allowed_values {
            // Empty value is allowed (default) or required (if allow_empty is false)
            if text.is_empty() {
                if !allowed.allow_empty {
                    return ValidationResult::warning("Value required");
                }
            } else if !allowed.matches(text) {
                return ValidationResult::error("Value must be one of the allowed options");
            }
        }

        ValidationResult::Valid
    }

    /// Check if any validation rules are configured
    pub fn has_validation(&self) -> bool {
        self.character_limits.is_some()
            || self.pattern_filters.is_some()
            || self.display_mask.is_some()
            || {
                #[cfg(feature = "validation")]
                {
                    self.custom_formatter.is_some()
                }
                #[cfg(not(feature = "validation"))]
                {
                    false
                }
            }
            || self.allowed_values.is_some()
    }

    /// Check if whitelist is configured
    pub fn has_allowed_values(&self) -> bool {
        self.allowed_values.is_some()
    }

    pub fn allows_field_switch(&self, text: &str) -> bool {
        // Character limits validation
        if let Some(ref limits) = self.character_limits {
            if !limits.allows_field_switch(text) {
                return false;
            }
        }

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

    /// Restrict content to one of the provided exact values (or empty).
    /// - Empty is considered valid by default.
    /// - Matching is case-sensitive by default.
    pub fn with_allowed_values<S>(mut self, values: Vec<S>) -> Self
    where
        S: Into<String>,
    {
        let vals: Vec<String> = values.into_iter().map(Into::into).collect();
        self.config.allowed_values = Some(AllowedValues::new(vals));
        self
    }

    /// Same as with_allowed_values, but case-insensitive (ASCII).
    pub fn with_allowed_values_ci<S>(mut self, values: Vec<S>) -> Self
    where
        S: Into<String>,
    {
        let vals: Vec<String> = values.into_iter().map(Into::into).collect();
        self.config.allowed_values = Some(AllowedValues::new(vals).case_insensitive(true));
        self
    }

    /// Configure whether empty value should be allowed when using AllowedValues.
    pub fn with_allowed_values_allow_empty(mut self, allow_empty: bool) -> Self {
        if let Some(av) = self.config.allowed_values.take() {
            self.config.allowed_values = Some(AllowedValues { allow_empty, ..av });
        } else {
            self.config.allowed_values = Some(AllowedValues::new(vec![]).allow_empty(allow_empty));
        }
        self
    }

    /// Enable or disable external validation indicator UI (feature 5)
    pub fn with_external_validation_enabled(mut self, enabled: bool) -> Self {
        self.config.external_validation_enabled = enabled;
        self
    }

    /// Build the final validation configuration
    pub fn build(self) -> ValidationConfig {
        self.config
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
        let config = ValidationConfigBuilder::new().with_max_length(10).build();

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
    fn test_allowed_values() {
        let config = ValidationConfigBuilder::new()
            .with_allowed_values(vec!["alpha", "beta", "gamma", "delta", "epsilon"])
            .build();

        // Empty should be valid by default
        let result = config.validate_content("");
        assert!(result.is_acceptable());

        // Exact allowed values are valid
        assert!(config.validate_content("alpha").is_acceptable());
        assert!(config.validate_content("beta").is_acceptable());

        // Anything else is an error
        let res = config.validate_content("alph");
        assert!(res.is_error());
        let res = config.validate_content("ALPHA");
        assert!(res.is_error()); // case-sensitive by default
    }

    #[test]
    fn test_allowed_values_case_insensitive_and_required() {
        let config = ValidationConfigBuilder::new()
            .with_allowed_values_ci(vec!["Yes", "No"])
            .with_allowed_values_allow_empty(false)
            .build();

        // Empty is not allowed now (warning so it's still acceptable for typing)
        let res = config.validate_content("");
        assert!(res.is_acceptable());

        // Case-insensitive matches
        assert!(config.validate_content("yes").is_acceptable());
        assert!(config.validate_content("NO").is_acceptable());

        // Random text is an error
        let res = config.validate_content("maybe");
        assert!(res.is_error());
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
        use crate::validation::{CharacterFilter, PatternFilters, PositionFilter, PositionRange};

        let patterns = PatternFilters::new().add_filter(PositionFilter::new(
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
