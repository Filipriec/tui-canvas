// src/autocomplete/state.rs
//! Simple autocomplete provider pattern - replaces complex trait

// Re-export the main types from data_provider for backward compatibility
pub use crate::data_provider::{AutocompleteProvider, SuggestionItem};

// Legacy compatibility - empty trait for migration
#[deprecated(note = "Use AutocompleteProvider instead")]
pub trait AutocompleteCanvasState {}
