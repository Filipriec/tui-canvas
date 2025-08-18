// src/textarea/mod.rs
//! Text area convenience exports.
//!
//! Re-export the core textarea types and provider so consumers can use
//! `canvas::textarea::TextArea` / `TextAreaState` / `TextAreaProvider`.

pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use provider::TextAreaProvider;
pub use state::{TextAreaEditor, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
pub use widget::TextArea;
