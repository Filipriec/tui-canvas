// src/editor/suggestions_stub.rs
// Crate-private no-op methods so internal calls compile when feature is off.

use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    pub(crate) fn open_suggestions(&mut self, _field_index: usize) {
        // no-op
    }

    pub(crate) fn close_suggestions(&mut self) {
        // no-op
    }

    pub(crate) fn handle_escape_readonly(&mut self) {
        // no-op
    }

    pub(crate) fn cancel_suggestions(&mut self) {
        // no-op
    }
}
