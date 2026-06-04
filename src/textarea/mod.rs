// src/textarea/mod.rs
//! Text area convenience exports.

mod actions;
mod input;
pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

#[cfg(feature = "syntect")]
pub mod highlight;

pub use provider::{TextAreaDataProvider, TextAreaProvider};
#[cfg(feature = "gui")]
pub use state::TextAreaLineNumberMode;
pub use state::{
    TextAreaEditor, TextAreaEventOutcome, TextAreaSearchMatch, TextAreaState, TextOverflowMode,
};
#[cfg(feature = "commandline")]
pub use state::TextAreaCommandLineState;

#[cfg(feature = "gui")]
pub use widget::TextArea;
