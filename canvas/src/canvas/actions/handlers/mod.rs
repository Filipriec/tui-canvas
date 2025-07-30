// src/canvas/actions/handlers/mod.rs

pub mod edit;
pub mod readonly;
pub mod highlight;

// Re-export handler functions
pub use edit::handle_edit_action;
pub use readonly::handle_readonly_action;
pub use highlight::handle_highlight_action;
