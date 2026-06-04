//! Stateful widgets and their widget-specific editing adapters.

#[cfg(feature = "textinput")]
pub mod textinput;

#[cfg(feature = "textarea")]
pub mod textarea;

#[cfg(feature = "gui")]
pub mod form;
