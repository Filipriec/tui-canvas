// src/textarea/mod.rs
pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

#[cfg(feature = "keymaps")]
pub mod commands_impl;

pub use provider::TextAreaProvider;
pub use state::{TextAreaEditor, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
pub use widget::TextArea;
