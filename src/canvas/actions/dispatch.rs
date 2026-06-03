// src/canvas/actions/dispatch.rs
//! Provides the typed dispatcher that maps CanvasAction → FormEditor method calls.

use super::types::{ActionResult, CanvasAction};
use crate::editor::FormEditor;
use crate::DataProvider;
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
            EnterEditMode => {
                self.enter_edit_mode();
                ActionResult::Success
            }
            EnterEditModeAfter => {
                self.enter_append_mode();
                ActionResult::Success
            }
            ExitEditMode => Self::into_action_result(self.exit_edit_mode()),
            EnterHighlightMode => {
                self.enter_highlight_mode();
                ActionResult::Success
            }
            EnterHighlightModeLinewise => {
                self.enter_highlight_line_mode();
                ActionResult::Success
            }
            ExitHighlightMode => {
                self.exit_highlight_mode();
                ActionResult::Success
            }

            // Movement
            MoveLeft => {
                if self.is_highlight_mode() {
                    self.move_left_with_selection();
                    ActionResult::Success
                } else {
                    Self::into_action_result(self.move_left())
                }
            }
            MoveRight => {
                if self.is_highlight_mode() {
                    self.move_right_with_selection();
                    ActionResult::Success
                } else {
                    Self::into_action_result(self.move_right())
                }
            }
            MoveUp => {
                if self.is_highlight_mode() {
                    self.move_up_with_selection();
                } else {
                    self.move_up();
                }
                ActionResult::Success
            }
            MoveDown => {
                if self.is_highlight_mode() {
                    self.move_down_with_selection();
                } else {
                    self.move_down();
                }
                ActionResult::Success
            }
            MoveWordNext => {
                if self.is_highlight_mode() {
                    self.move_word_next_with_selection();
                } else {
                    self.move_word_next();
                }
                ActionResult::Success
            }
            MoveWordPrev => {
                if self.is_highlight_mode() {
                    self.move_word_prev_with_selection();
                } else {
                    self.move_word_prev();
                }
                ActionResult::Success
            }
            MoveWordEnd => {
                if self.is_highlight_mode() {
                    self.move_word_end_with_selection();
                } else {
                    self.move_word_end();
                }
                ActionResult::Success
            }
            MoveWordEndPrev => {
                if self.is_highlight_mode() {
                    self.move_word_end_prev_with_selection();
                } else {
                    self.move_word_end_prev();
                }
                ActionResult::Success
            }
            MoveBigWordNext => {
                if self.is_highlight_mode() {
                    self.move_big_word_next_with_selection();
                } else {
                    self.move_big_word_next();
                }
                ActionResult::Success
            }
            MoveBigWordPrev => {
                if self.is_highlight_mode() {
                    self.move_big_word_prev_with_selection();
                } else {
                    self.move_big_word_prev();
                }
                ActionResult::Success
            }
            MoveBigWordEnd => {
                if self.is_highlight_mode() {
                    self.move_big_word_end_with_selection();
                } else {
                    self.move_big_word_end();
                }
                ActionResult::Success
            }
            MoveBigWordEndPrev => {
                if self.is_highlight_mode() {
                    self.move_big_word_end_prev_with_selection();
                } else {
                    self.move_big_word_end_prev();
                }
                ActionResult::Success
            }
            MoveFirstLine => Self::into_action_result(self.move_first_line()),
            MoveLastLine => Self::into_action_result(self.move_last_line()),
            MoveLineStart => {
                if self.is_highlight_mode() {
                    self.move_line_start_with_selection();
                } else {
                    self.move_line_start();
                }
                ActionResult::Success
            }
            MoveLineEnd => {
                if self.is_highlight_mode() {
                    self.move_line_end_with_selection();
                } else {
                    self.move_line_end();
                }
                ActionResult::Success
            }
            NextField => {
                self.next_field();
                ActionResult::Success
            }
            PrevField => {
                self.prev_field();
                ActionResult::Success
            }

            // Editing
            DeleteBackward => Self::into_action_result(self.delete_backward()),
            DeleteForward => Self::into_action_result(self.delete_forward()),
            Undo => {
                self.undo();
                ActionResult::Success
            }
            Redo => {
                self.redo();
                ActionResult::Success
            }
            OpenLineBelow => Self::into_action_result(self.open_line_below()),
            OpenLineAbove => Self::into_action_result(self.open_line_above()),

            // Suggestions
            #[cfg(feature = "suggestions")]
            TriggerSuggestions => {
                let _ = self.trigger_suggestions().map(|(idx, query)| {
                    let items = self.data_provider.fetch_suggestions_sync(idx, &query);
                    if items.is_empty() {
                        self.dismiss_suggestions();
                    } else {
                        self.apply_suggestions(items);
                    }
                });
                ActionResult::Success
            }
            #[cfg(feature = "suggestions")]
            SuggestionUp => {
                self.suggestions_prev();
                ActionResult::Success
            }
            #[cfg(feature = "suggestions")]
            SuggestionDown => {
                self.suggestions_next();
                ActionResult::Success
            }
            #[cfg(feature = "suggestions")]
            SelectSuggestion => {
                let _ = self.apply_suggestion();
                ActionResult::Success
            }
            #[cfg(feature = "suggestions")]
            ExitSuggestions => {
                self.dismiss_suggestions();
                ActionResult::Success
            }
            #[cfg(not(feature = "suggestions"))]
            TriggerSuggestions | SuggestionUp | SuggestionDown | SelectSuggestion
            | ExitSuggestions => ActionResult::Message("suggestions feature is disabled".into()),

            // Any actions that require arguments / not handled directly
            InsertChar(c) => Self::into_action_result(self.insert_char(c)),

            // Fallback: custom or unhandled
            Custom(name) => ActionResult::Message(format!("Unhandled custom action: {name}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::state::SelectionState;

    #[derive(Clone)]
    struct TestProvider {
        fields: Vec<(&'static str, String)>,
    }

    impl TestProvider {
        fn new(values: &[&'static str]) -> Self {
            Self {
                fields: values
                    .iter()
                    .enumerate()
                    .map(|(i, value)| {
                        let name = match i {
                            0 => "a",
                            1 => "b",
                            _ => "c",
                        };
                        (name, (*value).to_string())
                    })
                    .collect(),
            }
        }
    }

    impl DataProvider for TestProvider {
        fn field_count(&self) -> usize {
            self.fields.len()
        }

        fn field_name(&self, index: usize) -> &str {
            self.fields[index].0
        }

        fn field_value(&self, index: usize) -> &str {
            &self.fields[index].1
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            self.fields[index].1 = value;
        }
    }

    #[test]
    fn dispatch_extends_visual_selection_without_reanchoring() {
        let mut editor = FormEditor::new(TestProvider::new(&["alpha", "beta"]));

        assert!(editor.execute(CanvasAction::EnterHighlightMode).is_success());
        assert!(editor.execute(CanvasAction::MoveRight).is_success());
        assert_eq!(editor.current_field(), 0);
        assert_eq!(editor.cursor_position(), 1);
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        assert!(editor.execute(CanvasAction::MoveDown).is_success());
        assert_eq!(editor.current_field(), 1);
        assert!(matches!(
            editor.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));
    }
}
