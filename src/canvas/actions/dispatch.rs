// src/canvas/actions/dispatch.rs
//! Provides the typed dispatcher that maps CanvasAction → FormEditor method calls.

use crate::DataProvider;
use crate::editor::FormEditor;
use super::types::{CanvasAction, ActionResult};
use std::fmt::Display;

impl<D: DataProvider> FormEditor<D> {
    fn into_action_result<T, E: Display>(result: Result<T, E>) -> ActionResult {
        match result {
            Ok(_) => ActionResult::Success,
            Err(err) => ActionResult::Error(err.to_string()),
        }
    }

    /// Execute a CanvasAction on this editor instance.
    pub fn execute(&mut self, action: CanvasAction) -> ActionResult {
        use CanvasAction::*;
        match action {
            // Mode switching
            EnterEditMode => { self.enter_edit_mode(); ActionResult::Success }
            EnterEditModeAfter => { self.enter_append_mode(); ActionResult::Success }
            ExitEditMode => Self::into_action_result(self.exit_edit_mode()),
            EnterHighlightMode => { self.enter_highlight_mode(); ActionResult::Success }
            EnterHighlightModeLinewise => { self.enter_highlight_line_mode(); ActionResult::Success }
            ExitHighlightMode => { self.exit_highlight_mode(); ActionResult::Success }

            // Movement
            MoveLeft => Self::into_action_result(self.move_left()),
            MoveRight => Self::into_action_result(self.move_right()),
            MoveUp => { self.move_up(); ActionResult::Success }
            MoveDown => { self.move_down(); ActionResult::Success }
            MoveWordNext => { self.move_word_next(); ActionResult::Success }
            MoveWordPrev => { self.move_word_prev(); ActionResult::Success }
            MoveWordEnd => { self.move_word_end(); ActionResult::Success }
            MoveWordEndPrev => { self.move_word_end_prev(); ActionResult::Success }
            MoveFirstLine => Self::into_action_result(self.move_first_line()),
            MoveLastLine => Self::into_action_result(self.move_last_line()),
            MoveLineStart => { self.move_line_start(); ActionResult::Success }
            MoveLineEnd => { self.move_line_end(); ActionResult::Success }
            NextField => { self.next_field(); ActionResult::Success }
            PrevField => { self.prev_field(); ActionResult::Success }

            // Editing
            DeleteBackward => Self::into_action_result(self.delete_backward()),
            DeleteForward => Self::into_action_result(self.delete_forward()),
            OpenLineBelow => Self::into_action_result(self.open_line_below()),
            OpenLineAbove => Self::into_action_result(self.open_line_above()),

            // Suggestions
            TriggerSuggestions | SuggestionUp | SuggestionDown |
            SelectSuggestion | ExitSuggestions => ActionResult::HandledByApp("suggestion action".into()),

            // Any actions that require arguments / not handled directly
            InsertChar(c) => Self::into_action_result(self.insert_char(c)),

            // Fallback: custom or unhandled
            Custom(name) => ActionResult::Message(format!("Unhandled custom action: {}", name)),
        }
    }
}
