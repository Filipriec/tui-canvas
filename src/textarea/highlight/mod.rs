// src/textarea/highlight/mod.rs
#[cfg(all(feature = "syntect", feature = "gui"))]
pub mod engine;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub mod chunks;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub mod state;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub mod widget;

#[cfg(all(feature = "syntect", feature = "gui"))]
pub use engine::SyntectEngine;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub use chunks::StyledChunk;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub use state::TextAreaSyntaxState;
#[cfg(all(feature = "syntect", feature = "gui"))]
pub use widget::TextAreaSyntax;
