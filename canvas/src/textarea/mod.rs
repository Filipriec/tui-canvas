// src/textarea/mod.rs
pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use provider::TextAreaProvider;
pub use state::{TextAreaEditor, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
pub use widget::TextArea;
