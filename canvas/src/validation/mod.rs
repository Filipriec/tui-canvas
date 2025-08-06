// src/validation/mod.rs

// Core validation modules
pub mod config;
pub mod limits;
pub mod state;
pub mod patterns;
pub mod mask;  // Simple display mask instead of complex reserved chars
pub mod formatting; // Custom formatter and position mapping (feature 4)

// Re-export main types
pub use config::{ValidationConfig, ValidationResult, ValidationConfigBuilder};
pub use limits::{CharacterLimits, LimitCheckResult};
pub use state::{ValidationState, ValidationSummary};
pub use patterns::{PatternFilters, PositionFilter, PositionRange, CharacterFilter};
pub use mask::DisplayMask;  // Simple mask instead of ReservedCharacters
pub use formatting::{CustomFormatter, FormattingResult, PositionMapper, DefaultPositionMapper};

/// External validation UI state (Feature 5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalValidationState {
    NotValidated,
    Validating,
    Valid(Option<String>),
    Invalid { message: String, suggestion: Option<String> },
    Warning { message: String },
}

/// Validation error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Character limit exceeded: {message}")]
    LimitExceeded { message: String },
    
    #[error("Pattern validation failed: {message}")]
    PatternFailed { message: String },
    
    #[error("Custom validation failed: {message}")]
    CustomFailed { message: String },
}
