// src/editor/key_input.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;

#[cfg(feature = "keymap")]
use crate::keymap::{CanvasKeyAction, KeyEventOutcome, KeyStroke};
#[cfg(feature = "keymap")]
use crate::integration::focus_handoff::{
    BoundaryExit, key_outcome_for_vertical_navigation,
};

impl<D: DataProvider> FormEditor<D> {
    #[cfg(feature = "keymap")]
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        // Check if keymap exists first
        if self.keymap.is_none() {
            return KeyEventOutcome::NotMatched;
        }

        let mode = self.ui_state.current_mode;

        // Convert event to normalized stroke
        let stroke = KeyStroke {
            code: evt.code,
            modifiers: evt.modifiers,
        };

        // Add key to sequence tracker
        self.seq_tracker.add_key(stroke);

        // Look up the action in keymap
        let (matched, is_prefix) = {
            let km = self.keymap.as_ref().unwrap();
            km.lookup_action(mode, self.seq_tracker.sequence())
        };

        if let Some(action) = matched.cloned() {
            let outcome = self.dispatch_canvas_action(&action);
            self.seq_tracker.reset();
            return outcome;
        }

        if is_prefix {
            // Wait for more keys
            return KeyEventOutcome::Pending;
        }

        // No match: reset sequence and try insert-char fallback in Edit
        self.seq_tracker.reset();

        if mode == AppMode::Edit {
            if let KeyCode::Char(c) = evt.code {
                // Skip control/alt combos
                let m = evt.modifiers;
                let is_plain =
                    m.is_empty() || m == KeyModifiers::SHIFT;
                if is_plain {
                    if self.insert_char(c).is_ok() {
                        return KeyEventOutcome::Consumed(None);
                    }
                }
            }
        }

        KeyEventOutcome::NotMatched
    }

    #[cfg(feature = "keymap")]
    fn dispatch_canvas_action(
        &mut self,
        action: &CanvasKeyAction,
    ) -> KeyEventOutcome {
        match action {
            CanvasKeyAction::MoveLeft => {
                let _ = self.move_left();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveRight => {
                let _ = self.move_right();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveUp => {
                key_outcome_for_vertical_navigation(
                    self.move_up(),
                    BoundaryExit::Top,
                )
            }
            CanvasKeyAction::MoveDown => {
                key_outcome_for_vertical_navigation(
                    self.move_down(),
                    BoundaryExit::Bottom,
                )
            }
            CanvasKeyAction::NextField => {
                key_outcome_for_vertical_navigation(
                    self.next_field(),
                    BoundaryExit::Bottom,
                )
            }
            CanvasKeyAction::PrevField => {
                key_outcome_for_vertical_navigation(
                    self.prev_field(),
                    BoundaryExit::Top,
                )
            }
            CanvasKeyAction::MoveLineStart => {
                self.move_line_start();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveLineEnd => {
                self.move_line_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveFirstLine => {
                let _ = self.move_first_line();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveLastLine => {
                let _ = self.move_last_line();
                KeyEventOutcome::Consumed(None)
            }

            CanvasKeyAction::MoveWordNext => {
                self.move_word_next();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordPrev => {
                self.move_word_prev();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordEnd => {
                self.move_word_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordEndPrev => {
                self.move_word_end_prev();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordNext => {
                self.move_big_word_next();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordPrev => {
                self.move_big_word_prev();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordEnd => {
                self.move_big_word_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordEndPrev => {
                self.move_big_word_end_prev();
                KeyEventOutcome::Consumed(None)
            }

            CanvasKeyAction::DeleteCharBackward => {
                let _ = self.delete_backward();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteCharForward => {
                let _ = self.delete_forward();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineBelow => {
                let _ = self.open_line_below();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineAbove => {
                let _ = self.open_line_above();
                KeyEventOutcome::Consumed(None)
            }

            #[cfg(feature = "suggestions")]
            CanvasKeyAction::OpenSuggestions => {
                let idx = self.current_field();
                self.open_suggestions(idx);
                KeyEventOutcome::Consumed(None)
            }
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::ApplySuggestion | CanvasKeyAction::EnterDecider => {
                if let Some(_applied) = self.apply_suggestion() {
                    KeyEventOutcome::Consumed(None)
                } else {
                    KeyEventOutcome::Consumed(None)
                }
            }
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::SuggestionDown => {
                self.suggestions_next();
                KeyEventOutcome::Consumed(None)
            }
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::SuggestionUp => {
                self.suggestions_prev();
                KeyEventOutcome::Consumed(None)
            }

            CanvasKeyAction::EnterEditModeBefore => {
                self.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeAfter => {
                // Move forward 1 char if possible (vim 'a'), then enter insert
                let txt_len = self.current_text().chars().count();
                let pos = self.ui_state.cursor_pos;
                if pos < txt_len {
                    self.ui_state.cursor_pos = pos + 1;
                    self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
                }
                self.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::Exit | CanvasKeyAction::ExitEditMode => {
                let _ = self.exit_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterHighlightMode => {
                self.enter_highlight_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterHighlightModeLinewise => {
                self.enter_highlight_line_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExitHighlightMode => {
                self.exit_highlight_mode();
                KeyEventOutcome::Consumed(None)
            }

            CanvasKeyAction::Unknown(_) => KeyEventOutcome::NotMatched,
            #[cfg(not(feature = "suggestions"))]
            CanvasKeyAction::OpenSuggestions
            | CanvasKeyAction::ApplySuggestion
            | CanvasKeyAction::EnterDecider
            | CanvasKeyAction::SuggestionDown
            | CanvasKeyAction::SuggestionUp => KeyEventOutcome::NotMatched,
        }
    }
}
