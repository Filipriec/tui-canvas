// src/textarea/highlight/mod.rs
#[cfg(feature = "syntect")]
pub mod chunks;
#[cfg(feature = "syntect")]
pub mod engine;
#[cfg(feature = "syntect")]
pub mod state;
#[cfg(feature = "syntect")]
pub mod widget;

#[cfg(feature = "syntect")]
pub use chunks::StyledChunk;
#[cfg(feature = "syntect")]
pub use engine::SyntectEngine;
#[cfg(feature = "syntect")]
pub use state::TextAreaSyntaxState;
#[cfg(feature = "syntect")]
pub use widget::TextAreaSyntax;
