// src/validation/mod.rs
//! Validation module for canvas form fields

pub mod config;
pub mod limits;
pub mod state;
pub mod patterns;

// Re-export main types
pub use config::{ValidationConfig, ValidationResult, ValidationConfigBuilder};
pub use limits::{CharacterLimits, LimitCheckResult};
pub use state::{ValidationState, ValidationSummary};
pub use patterns::{PatternFilters, PositionFilter, PositionRange, CharacterFilter};

/// Validation error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Character limit exceeded: {current}/{max}")]
    CharacterLimitExceeded { current: usize, max: usize },
    
    #[error("Invalid character '{char}' at position {position}")]
    InvalidCharacter { char: char, position: usize },
    
    #[error("Pattern validation failed: {message}")]
    PatternValidationFailed { message: String },
    
    #[error("Validation configuration error: {message}")]
    ConfigurationError { message: String },
}

/// Result type for validation operations
pub type Result<T> = std::result::Result<T, ValidationError>;
