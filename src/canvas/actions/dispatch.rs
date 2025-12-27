// src/canvas/actions/dispatch.rs
//! Provides the typed dispatcher that maps CanvasAction → FormEditor method calls.

use crate::DataProvider;
use crate::editor::FormEditor;
use super::types::{CanvasAction, ActionResult};

impl<D: DataProvider> FormEditor<D> {
    /// Execute a CanvasAction on this editor instance.
    pub fn execute(&mut self, action: CanvasAction) -> ActionResult {
        use CanvasAction::*;
        match action {
            // ---- Mode switching ----
            EnterEditMode => { self.enter_edit_mode(); ActionResult::Success }
            EnterEditModeAfter => { self.enter_append_mode(); ActionResult::Success }
            ExitEditMode => { let _ = self.exit_edit_mode(); ActionResult::Success }
            EnterHighlightMode => { self.enter_highlight_mode(); ActionResult::Success }
            EnterHighlightModeLinewise => { self.enter_highlight_line_mode(); ActionResult::Success }
            ExitHighlightMode => { self.exit_highlight_mode(); ActionResult::Success }

            // ---- Movement ----
            MoveLeft => { self.move_left(); ActionResult::Success }
            MoveRight => { self.move_right(); ActionResult::Success }
            MoveUp => { self.move_up(); ActionResult::Success }
            MoveDown => { self.move_down(); ActionResult::Success }
            MoveWordNext => { self.move_word_next(); ActionResult::Success }
            MoveWordPrev => { self.move_word_prev(); ActionResult::Success }
            MoveWordEnd => { self.move_word_end(); ActionResult::Success }
            MoveWordEndPrev => { self.move_word_end_prev(); ActionResult::Success }
            MoveFirstLine => { self.move_first_line(); ActionResult::Success }
            MoveLastLine => { self.move_last_line(); ActionResult::Success }
            MoveLineStart => { self.move_line_start(); ActionResult::Success }
            MoveLineEnd => { self.move_line_end(); ActionResult::Success }
            NextField => { self.next_field(); ActionResult::Success }
            PrevField => { self.prev_field(); ActionResult::Success }

            // ---- Editing ----
            DeleteBackward => { let _ = self.delete_backward(); ActionResult::Success }
            DeleteForward => { let _ = self.delete_forward(); ActionResult::Success }
            OpenLineBelow => { let _ = self.open_line_below(); ActionResult::Success }
            OpenLineAbove => { let _ = self.open_line_above(); ActionResult::Success }

            // ---- Suggestions ----
            TriggerSuggestions | SuggestionUp | SuggestionDown |
            SelectSuggestion | ExitSuggestions => ActionResult::HandledByApp("suggestion action".into()),

            // ---- Any actions that require arguments / not handled directly ----
            InsertChar(c) => { let _ = self.insert_char(c); ActionResult::Success }

            // ---- Fallback: custom or unhandled ----
            Custom(name) => ActionResult::Message(format!("Unhandled custom action: {}", name)),
        }
    }
}
