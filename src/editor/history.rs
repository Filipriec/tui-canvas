// src/editor/history.rs
//! Snapshot-based undo/redo history for [`FormEditor`].
//!
//! See `analysis/UNDO_REDO_DESIGN.md` for the rationale. In short: each
//! checkpoint stores the full editable content (via
//! [`DataProvider::capture_content`]) plus the caret, which is robust against
//! the mask/validation/rope edit paths that an inverse-op log would have to
//! track. Consecutive same-kind edits coalesce into one undo step.

use crate::canvas::state::SelectionState;
use crate::editor::FormEditor;
use crate::DataProvider;

/// Default number of undo steps retained before the oldest is dropped.
pub(crate) const DEFAULT_HISTORY_LIMIT: usize = 100;

/// The kind of an edit, used to coalesce consecutive same-kind edits into a
/// single undo step (e.g. a run of typed characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditKind {
    Insert,
    Delete,
    /// Structural / bulk edits that never coalesce (paste, set-field, clear).
    Other,
}

impl EditKind {
    fn coalesces(self) -> bool {
        matches!(self, EditKind::Insert | EditKind::Delete)
    }
}

/// A point-in-time snapshot of the editable state.
#[derive(Debug, Clone)]
pub(crate) struct EditSnapshot {
    content: Vec<String>,
    current_field: usize,
    cursor_pos: usize,
}

impl<D: DataProvider> FormEditor<D> {
    fn snapshot_now(&self) -> EditSnapshot {
        EditSnapshot {
            content: self.data_provider.capture_content(),
            current_field: self.ui_state.current_field,
            cursor_pos: self.ui_state.cursor_pos,
        }
    }

    /// Record a pre-mutation checkpoint. Call this immediately *before* applying
    /// a content change so the snapshot captures the state to return to.
    ///
    /// Consecutive edits of the same coalescible kind extend the current run
    /// instead of pushing a new undo step.
    pub(crate) fn record_checkpoint(&mut self, kind: EditKind) {
        if !self.history_enabled {
            return;
        }

        // Continue an in-progress run of the same kind: the run's undo target
        // was already captured by the first checkpoint in the run.
        if kind.coalesces() && self.history_last_kind == Some(kind) {
            return;
        }

        let snapshot = self.snapshot_now();
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > self.history_limit {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.history_last_kind = Some(kind);
    }

    /// End the current coalescing run so the next edit starts a fresh undo step.
    /// Called on navigation / mode changes.
    pub(crate) fn break_undo_coalescing(&mut self) {
        self.history_last_kind = None;
    }

    fn apply_snapshot(&mut self, snapshot: EditSnapshot) {
        self.data_provider.restore_content(&snapshot.content);

        let field_count = self.data_provider.field_count();
        self.ui_state.current_field = if field_count == 0 {
            0
        } else {
            snapshot.current_field.min(field_count - 1)
        };

        let len = self.current_text().chars().count();
        self.set_cursor_raw(snapshot.cursor_pos.min(len));
        self.ui_state.selection = SelectionState::None;

        self.after_history_restore();
    }

    /// Re-sync derived state after a restore.
    ///
    /// Validation results are recomputed from the restored content and the
    /// suggestions dropdown is dismissed. Computed fields are *not* recomputed
    /// here: the editor does not retain the user's `ComputedProvider`, so a host
    /// using computed fields should call its recompute path after `undo`/`redo`.
    fn after_history_restore(&mut self) {
        #[cfg(feature = "validation")]
        {
            let count = self.data_provider.field_count();
            for i in 0..count {
                let text = self.data_provider.field_value(i).to_string();
                let _ = self.ui_state.validation.validate_field_content(i, &text);
            }
        }
        #[cfg(feature = "suggestions")]
        self.ui_state.close_suggestions();
    }

    /// Undo the most recent edit (or run). Returns `false` if there is nothing
    /// to undo.
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            let current = self.snapshot_now();
            self.redo_stack.push(current);
            self.apply_snapshot(previous);
            self.history_last_kind = None;
            true
        } else {
            false
        }
    }

    /// Redo the most recently undone edit. Returns `false` if there is nothing
    /// to redo.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.snapshot_now();
            self.undo_stack.push(current);
            self.apply_snapshot(next);
            self.history_last_kind = None;
            true
        } else {
            false
        }
    }

    /// Whether there is any edit to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether there is any undone edit to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all undo/redo history.
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.history_last_kind = None;
    }

    /// Set the maximum number of retained undo steps (oldest dropped first).
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit.max(1);
        while self.undo_stack.len() > self.history_limit {
            self.undo_stack.remove(0);
        }
    }

    /// Enable or disable undo/redo history capture (enabled by default).
    ///
    /// While disabled, edits do not record checkpoints, so apps that don't want
    /// undo — or want to avoid the per-edit snapshot cost — can switch it off.
    /// Existing history is cleared when disabling.
    pub fn set_history_enabled(&mut self, enabled: bool) {
        self.history_enabled = enabled;
        if !enabled {
            self.clear_history();
        }
    }

    /// Whether undo/redo history capture is currently enabled.
    pub fn is_history_enabled(&self) -> bool {
        self.history_enabled
    }
}
