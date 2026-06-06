#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::actions::selection::helix::HelixCase,
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn dispatch_textarea_key_action_helix(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        if let Some(outcome) = self.dispatch_shared_textarea_key_action(action, count) {
            return outcome;
        }

        match action {
            CanvasKeyAction::MoveUp => {
                self.move_visual_line_helix(false, count);
                if self.mode() == AppMode::Nor {
                    self.collapse_helix_selection_to_cursor();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveDown => {
                self.move_visual_line_helix(true, count);
                if self.mode() == AppMode::Nor {
                    self.collapse_helix_selection_to_cursor();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteSelection => {
                self.delete_selection_helix(true, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteSelectionNoYank => {
                self.delete_selection_helix(false, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeSelection => {
                self.change_selection_helix(true, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeSelectionNoYank => {
                self.change_selection_helix(false, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::YankSelection => {
                for _ in 0..count {
                    self.yank_primary_selection_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::CollapseSelection => {
                self.collapse_selection_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SearchNext => {
                self.search_next_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SearchPrev => {
                self.search_prev_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SelectAll => {
                self.select_all_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::FlipSelections => {
                self.flip_selection_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SwitchCase => {
                self.switch_case_selection_helix(HelixCase::Toggle);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SwitchToLowercase => {
                self.switch_case_selection_helix(HelixCase::Lower);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SwitchToUppercase => {
                self.switch_case_selection_helix(HelixCase::Upper);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::TrimSelections => {
                self.trim_selection_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::GotoFirstNonWhitespace => {
                self.goto_first_nonwhitespace_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendLineBelow => {
                for _ in 0..count {
                    self.extend_line_below_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendToLineBounds => {
                for _ in 0..count {
                    self.extend_to_line_bounds_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteAfter => {
                self.paste_after_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_before_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordNext => {
                self.select_next_word_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordPrev => {
                self.select_prev_word_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordEnd => {
                self.select_word_end_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveWordEndPrev => {
                self.select_word_end_prev_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordNext => {
                self.select_next_big_word_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordPrev => {
                self.select_prev_big_word_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordEnd => {
                self.select_big_word_end_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveBigWordEndPrev => {
                self.select_big_word_end_prev_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            _ => {
                let outcome = self.execute_canvas_key_action(action, count);
                if self.mode() == AppMode::Nor {
                    if let Some(canvas_action) = action.to_canvas_action() {
                        if canvas_action.is_movement_action() {
                            self.collapse_helix_selection_to_cursor();
                        }
                    }
                }
                outcome
            }
        }
    }
}
