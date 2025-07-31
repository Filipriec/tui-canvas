// src/canvas/actions/handlers/mod.rs

pub mod edit;
pub mod readonly;
pub mod highlight;
pub mod dispatcher;

pub use edit::handle_edit_action;
pub use readonly::handle_readonly_action;
pub use highlight::handle_highlight_action;
pub use dispatcher::dispatch_action;
