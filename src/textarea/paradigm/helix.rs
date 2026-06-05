#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
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
