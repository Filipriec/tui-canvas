// src/textarea/mod.rs
//! Text area convenience exports.

pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

#[cfg(feature = "syntect")]
pub mod highlight;

pub use provider::{TextAreaDataProvider, TextAreaProvider};
pub use state::{TextAreaEditor, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
pub use widget::TextArea;
