// src/config/mod.rs

mod registry;
mod config;
mod validation;
pub mod introspection;

// Re-export everything from the main config module
pub use registry::*;
pub use validation::*;
pub use config::*;
pub use introspection::*;
