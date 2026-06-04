//! Editor-owned input adapters.

#[cfg(feature = "keybindings")]
pub mod keybindings;

#[cfg(feature = "crossterm")]
pub mod normal;
