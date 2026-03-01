// src/editor/key_input.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;

#[cfg(feature = "keymap")]
use crate::keymap::{KeyEventOutcome, KeyStroke};
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
            km.lookup(mode, self.seq_tracker.sequence())
        };

        if let Some(action) = matched {
            // Clone the action string to avoid borrow checker issues
            let action_owned = action.to_string();
            let outcome = self.dispatch_canvas_action(&action_owned);
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
    fn dispatch_canvas_action(&mut self, action: &str) -> KeyEventOutcome {
        match action {
            // Movement
            "move_left" => {
                let _ = self.move_left();
                KeyEventOutcome::Consumed(None)
            }
            "move_right" => {
                let _ = self.move_right();
                KeyEventOutcome::Consumed(None)
            }
            "move_up" => {
                key_outcome_for_vertical_navigation(
                    self.move_up(),
                    BoundaryExit::Top,
                )
            }
            "move_down" => {
                key_outcome_for_vertical_navigation(
                    self.move_down(),
                    BoundaryExit::Bottom,
                )
            }
            "next_field" => {
                key_outcome_for_vertical_navigation(
                    self.next_field(),
                    BoundaryExit::Bottom,
                )
            }
            "prev_field" => {
                key_outcome_for_vertical_navigation(
                    self.prev_field(),
                    BoundaryExit::Top,
                )
            }
            "move_line_start" => {
                self.move_line_start();
                KeyEventOutcome::Consumed(None)
            }
            "move_line_end" => {
                self.move_line_end();
                KeyEventOutcome::Consumed(None)
            }
            "move_first_line" => {
                let _ = self.move_first_line();
                KeyEventOutcome::Consumed(None)
            }
            "move_last_line" => {
                let _ = self.move_last_line();
                KeyEventOutcome::Consumed(None)
            }

            // Word/big-word movement (cross-field aware)
            "move_word_next" => {
                self.move_word_next();
                KeyEventOutcome::Consumed(None)
            }
            "move_word_prev" => {
                self.move_word_prev();
                KeyEventOutcome::Consumed(None)
            }
            "move_word_end" => {
                self.move_word_end();
                KeyEventOutcome::Consumed(None)
            }
            "move_word_end_prev" => {
                self.move_word_end_prev();
                KeyEventOutcome::Consumed(None)
            }
            "move_big_word_next" => {
                self.move_big_word_next();
                KeyEventOutcome::Consumed(None)
            }
            "move_big_word_prev" => {
                self.move_big_word_prev();
                KeyEventOutcome::Consumed(None)
            }
            "move_big_word_end" => {
                self.move_big_word_end();
                KeyEventOutcome::Consumed(None)
            }
            "move_big_word_end_prev" => {
                self.move_big_word_end_prev();
                KeyEventOutcome::Consumed(None)
            }

            // Editing
            "delete_char_backward" => {
                let _ = self.delete_backward();
                KeyEventOutcome::Consumed(None)
            }
            "delete_char_forward" => {
                let _ = self.delete_forward();
                KeyEventOutcome::Consumed(None)
            }
            "open_line_below" => {
                let _ = self.open_line_below();
                KeyEventOutcome::Consumed(None)
            }
            "open_line_above" => {
                let _ = self.open_line_above();
                KeyEventOutcome::Consumed(None)
            }

            // Suggestions (only when feature is enabled)
            #[cfg(feature = "suggestions")]
            "open_suggestions" => {
                let idx = self.current_field();
                self.open_suggestions(idx);
                KeyEventOutcome::Consumed(None)
            }
            #[cfg(feature = "suggestions")]
            "apply_suggestion" | "enter_decider" => {
                if let Some(_applied) = self.apply_suggestion() {
                    KeyEventOutcome::Consumed(None)
                } else {
                    KeyEventOutcome::Consumed(None)
                }
            }
            #[cfg(feature = "suggestions")]
            "suggestion_down" => {
                self.suggestions_next();
                KeyEventOutcome::Consumed(None)
            }
            #[cfg(feature = "suggestions")]
            "suggestion_up" => {
                self.suggestions_prev();
                KeyEventOutcome::Consumed(None)
            }

            // Mode transitions (vim-like)
            "enter_edit_mode_before" => {
                self.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            "enter_edit_mode_after" => {
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
            "exit" | "exit_edit_mode" => {
                let _ = self.exit_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            "enter_highlight_mode" => {
                self.enter_highlight_mode();
                KeyEventOutcome::Consumed(None)
            }
            "enter_highlight_mode_linewise" => {
                self.enter_highlight_line_mode();
                KeyEventOutcome::Consumed(None)
            }
            "exit_highlight_mode" => {
                self.exit_highlight_mode();
                KeyEventOutcome::Consumed(None)
            }

            _ => KeyEventOutcome::NotMatched,
        }
    }
}
