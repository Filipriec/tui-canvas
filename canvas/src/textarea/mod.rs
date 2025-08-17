// src/textarea/mod.rs
// Module routing and re-exports only. No logic here.

pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use provider::TextAreaProvider;
pub use state::{TextAreaEditor, TextAreaState};

#[cfg(feature = "gui")]
pub use widget::TextArea;
