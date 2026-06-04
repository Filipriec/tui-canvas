//! Optional and support features for the editor engine.

pub mod history;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "computed")]
pub mod computed;
