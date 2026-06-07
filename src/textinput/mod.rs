//! Single-line text input convenience exports.

pub mod provider;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use provider::{TextInputDataProvider, TextInputProvider};
pub use state::{TextInputEventOutcome, TextInputState};

#[cfg(feature = "gui")]
pub use widget::TextInput;
