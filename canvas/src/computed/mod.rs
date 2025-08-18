//! Computed fields subsystem.
//!
//! This module exposes the provider trait and the internal state management
//! for computed (display-only) fields. Computed fields are values derived
//! from other fields in the form and are not directly editable by the user.

pub mod provider;
pub mod state;

pub use provider::{ComputedContext, ComputedProvider};
pub use state::ComputedState;
