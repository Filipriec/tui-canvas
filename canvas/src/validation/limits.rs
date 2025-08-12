// src/validation/limits.rs
//! Character limits validation implementation

use crate::validation::ValidationResult;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

/// Character limits configuration for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterLimits {
    /// Maximum number of characters allowed (None = unlimited)
    max_length: Option<usize>,
    
    /// Minimum number of characters required (None = no minimum)
    min_length: Option<usize>,
    
    /// Warning threshold (warn when approaching max limit)
    warning_threshold: Option<usize>,
    
    /// Count mode: characters vs display width
    count_mode: CountMode,
}

/// How to count characters for limit checking
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CountMode {
    /// Count actual characters (default)
    Characters,
    
    /// Count display width (useful for CJK characters)
    DisplayWidth,
    
    /// Count bytes (rarely used, but available)
    Bytes,
}

impl Default for CountMode {
    fn default() -> Self {
        CountMode::Characters
    }
}

/// Result of a character limit check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitCheckResult {
    /// Within limits
    Ok,
    
    /// Approaching limit (warning)
    Warning { current: usize, max: usize },
    
    /// At or exceeding limit (error)
    Exceeded { current: usize, max: usize },
    
    /// Below minimum length
    TooShort { current: usize, min: usize },
}

impl CharacterLimits {
    /// Create new character limits with just max length
    pub fn new(max_length: usize) -> Self {
        Self {
            max_length: Some(max_length),
            min_length: None,
            warning_threshold: None,
            count_mode: CountMode::default(),
        }
    }
    
    /// Create new character limits with min and max
    pub fn new_range(min_length: usize, max_length: usize) -> Self {
        Self {
            max_length: Some(max_length),
            min_length: Some(min_length),
            warning_threshold: None,
            count_mode: CountMode::default(),
        }
    }
    
    /// Set warning threshold (when to show warning before hitting limit)
    pub fn with_warning_threshold(mut self, threshold: usize) -> Self {
        self.warning_threshold = Some(threshold);
        self
    }
    
    /// Set count mode (characters vs display width vs bytes)
    pub fn with_count_mode(mut self, mode: CountMode) -> Self {
        self.count_mode = mode;
        self
    }
    
    /// Get maximum length
    pub fn max_length(&self) -> Option<usize> {
        self.max_length
    }
    
    /// Get minimum length
    pub fn min_length(&self) -> Option<usize> {
        self.min_length
    }
    
    /// Get warning threshold
    pub fn warning_threshold(&self) -> Option<usize> {
        self.warning_threshold
    }
    
    /// Get count mode
    pub fn count_mode(&self) -> CountMode {
        self.count_mode
    }
    
    /// Count characters/width/bytes according to the configured mode
    fn count(&self, text: &str) -> usize {
        match self.count_mode {
            CountMode::Characters => text.chars().count(),
            CountMode::DisplayWidth => text.width(),
            CountMode::Bytes => text.len(),
        }
    }
    
    /// Check if inserting a character would exceed limits
    pub fn validate_insertion(
        &self,
        current_text: &str,
        position: usize,
        character: char,
    ) -> Option<ValidationResult> {
        let current_count = self.count(current_text);
        let char_count = match self.count_mode {
            CountMode::Characters => 1,
            CountMode::DisplayWidth => {
                let char_str = character.to_string();
                char_str.width()
            },
            CountMode::Bytes => character.len_utf8(),
        };
        let new_count = current_count + char_count;
        
        // Check max length
        if let Some(max) = self.max_length {
            if new_count > max {
                return Some(ValidationResult::error(format!(
                    "Character limit exceeded: {}/{}", 
                    new_count, 
                    max
                )));
            }
            
            // Check warning threshold
            if let Some(warning_threshold) = self.warning_threshold {
                if new_count >= warning_threshold && current_count < warning_threshold {
                    return Some(ValidationResult::warning(format!(
                        "Approaching character limit: {}/{}", 
                        new_count, 
                        max
                    )));
                }
            }
        }
        
        None // No validation issues
    }
    
    /// Validate the current content
    pub fn validate_content(&self, text: &str) -> Option<ValidationResult> {
        let count = self.count(text);
        
        // Check minimum length
        if let Some(min) = self.min_length {
            if count < min {
                return Some(ValidationResult::warning(format!(
                    "Minimum length not met: {}/{}", 
                    count, 
                    min
                )));
            }
        }
        
        // Check maximum length
        if let Some(max) = self.max_length {
            if count > max {
                return Some(ValidationResult::error(format!(
                    "Character limit exceeded: {}/{}", 
                    count, 
                    max
                )));
            }
            
            // Check warning threshold
            if let Some(warning_threshold) = self.warning_threshold {
                if count >= warning_threshold {
                    return Some(ValidationResult::warning(format!(
                        "Approaching character limit: {}/{}", 
                        count, 
                        max
                    )));
                }
            }
        }
        
        None // No validation issues
    }
    
    /// Get the current status of the text against limits
    pub fn check_limits(&self, text: &str) -> LimitCheckResult {
        let count = self.count(text);
        
        // Check max length first
        if let Some(max) = self.max_length {
            if count > max {
                return LimitCheckResult::Exceeded { current: count, max };
            }
            
            // Check warning threshold
            if let Some(warning_threshold) = self.warning_threshold {
                if count >= warning_threshold {
                    return LimitCheckResult::Warning { current: count, max };
                }
            }
        }
        
        // Check min length
        if let Some(min) = self.min_length {
            if count < min {
                return LimitCheckResult::TooShort { current: count, min };
            }
        }
        
        LimitCheckResult::Ok
    }
    
    /// Get a human-readable status string
    pub fn status_text(&self, text: &str) -> Option<String> {
        match self.check_limits(text) {
            LimitCheckResult::Ok => {
                // Show current/max if we have a max limit
                if let Some(max) = self.max_length {
                    Some(format!("{}/{}", self.count(text), max))
                } else {
                    None
                }
            },
            LimitCheckResult::Warning { current, max } => {
                Some(format!("{}/{} (approaching limit)", current, max))
            },
            LimitCheckResult::Exceeded { current, max } => {
                Some(format!("{}/{} (exceeded)", current, max))
            },
            LimitCheckResult::TooShort { current, min } => {
                Some(format!("{}/{} minimum", current, min))
            },
        }
    }
    pub fn allows_field_switch(&self, text: &str) -> bool {
        if let Some(min) = self.min_length {
            let count = self.count(text);
            // Allow switching if field is empty OR meets minimum requirement
            count == 0 || count >= min
        } else {
            true // No minimum requirement, always allow switching
        }
    }
    
    /// Get reason why field switching is not allowed (if any)
    pub fn field_switch_block_reason(&self, text: &str) -> Option<String> {
        if let Some(min) = self.min_length {
            let count = self.count(text);
            if count > 0 && count < min {
                return Some(format!(
                    "Field must be empty or have at least {} characters (currently: {})",
                    min, count
                ));
            }
        }
        None
    }
}

impl Default for CharacterLimits {
    fn default() -> Self {
        Self {
            max_length: Some(30), // Default 30 character limit as specified
            min_length: None,
            warning_threshold: None,
            count_mode: CountMode::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_limits_creation() {
        let limits = CharacterLimits::new(10);
        assert_eq!(limits.max_length(), Some(10));
        assert_eq!(limits.min_length(), None);
        
        let range_limits = CharacterLimits::new_range(5, 15);
        assert_eq!(range_limits.min_length(), Some(5));
        assert_eq!(range_limits.max_length(), Some(15));
    }
    
    #[test]
    fn test_default_limits() {
        let limits = CharacterLimits::default();
        assert_eq!(limits.max_length(), Some(30));
    }
    
    #[test]
    fn test_character_counting() {
        let limits = CharacterLimits::new(5);
        
        // Test character mode (default)
        assert_eq!(limits.count("hello"), 5);
        assert_eq!(limits.count("héllo"), 5); // Accented character counts as 1
        
        // Test display width mode
        let limits = limits.with_count_mode(CountMode::DisplayWidth);
        assert_eq!(limits.count("hello"), 5);
        
        // Test bytes mode
        let limits = limits.with_count_mode(CountMode::Bytes);
        assert_eq!(limits.count("hello"), 5);
        assert_eq!(limits.count("héllo"), 6); // é takes 2 bytes in UTF-8
    }
    
    #[test]
    fn test_insertion_validation() {
        let limits = CharacterLimits::new(5);
        
        // Valid insertion
        let result = limits.validate_insertion("test", 4, 'x');
        assert!(result.is_none()); // No validation issues
        
        // Invalid insertion (would exceed limit)
        let result = limits.validate_insertion("tests", 5, 'x');
        assert!(result.is_some());
        assert!(!result.unwrap().is_acceptable());
    }
    
    #[test]
    fn test_content_validation() {
        let limits = CharacterLimits::new_range(3, 10);
        
        // Too short
        let result = limits.validate_content("hi");
        assert!(result.is_some());
        assert!(result.unwrap().is_acceptable()); // Warning, not error
        
        // Just right
        let result = limits.validate_content("hello");
        assert!(result.is_none());
        
        // Too long
        let result = limits.validate_content("hello world!");
        assert!(result.is_some());
        assert!(!result.unwrap().is_acceptable()); // Error
    }
    
    #[test]
    fn test_warning_threshold() {
        let limits = CharacterLimits::new(10).with_warning_threshold(8);
        
        // Below warning threshold
        let result = limits.validate_insertion("1234567", 7, 'x');
        assert!(result.is_none());
        
        // At warning threshold
        let result = limits.validate_insertion("1234567", 7, 'x');
        assert!(result.is_none()); // This brings us to 8 chars
        
        let result = limits.validate_insertion("12345678", 8, 'x');
        assert!(result.is_some());
        assert!(result.unwrap().is_acceptable()); // Warning, not error
    }
    
    #[test]
    fn test_status_text() {
        let limits = CharacterLimits::new(10);
        
        assert_eq!(limits.status_text("hello"), Some("5/10".to_string()));
        
        let limits = limits.with_warning_threshold(8);
        assert_eq!(limits.status_text("12345678"), Some("8/10 (approaching limit)".to_string()));
        assert_eq!(limits.status_text("1234567890x"), Some("11/10 (exceeded)".to_string()));
    }
    
    #[test]
    fn test_field_switch_blocking() {
        let limits = CharacterLimits::new_range(3, 10);
        
        // Empty field: should allow switching
        assert!(limits.allows_field_switch(""));
        assert!(limits.field_switch_block_reason("").is_none());
        
        // Field with content below minimum: should block switching
        assert!(!limits.allows_field_switch("hi"));
        assert!(limits.field_switch_block_reason("hi").is_some());
        assert!(limits.field_switch_block_reason("hi").unwrap().contains("at least 3 characters"));
        
        // Field meeting minimum: should allow switching
        assert!(limits.allows_field_switch("hello"));
        assert!(limits.field_switch_block_reason("hello").is_none());
        
        // Field exceeding maximum: should still allow switching (validation shows error but doesn't block)
        assert!(limits.allows_field_switch("this is way too long"));
        assert!(limits.field_switch_block_reason("this is way too long").is_none());
    }
    
    #[test]
    fn test_field_switch_no_minimum() {
        let limits = CharacterLimits::new(10); // Only max, no minimum
        
        // Should always allow switching when there's no minimum
        assert!(limits.allows_field_switch(""));
        assert!(limits.allows_field_switch("a"));
        assert!(limits.allows_field_switch("hello"));
        
        assert!(limits.field_switch_block_reason("").is_none());
        assert!(limits.field_switch_block_reason("a").is_none());
    }
}
