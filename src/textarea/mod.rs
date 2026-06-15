// src/textarea/mod.rs
//! Text area convenience exports.

mod actions;
mod input;
#[cfg(feature = "keybindings")]
mod paradigm;
pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

#[cfg(feature = "syntect")]
pub mod highlight;

pub use provider::{TextAreaDataProvider, TextAreaProvider};
#[cfg(feature = "commandline")]
pub use state::TextAreaCommandLineState;
#[cfg(feature = "gui")]
pub use state::{ParseTextAreaLineNumberModeError, TextAreaLineNumberMode};
pub use state::{TextAreaEventOutcome, TextAreaSearchMatch, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
pub use widget::TextArea;
