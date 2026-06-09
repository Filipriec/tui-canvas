#[cfg(feature = "keybindings")]
use crate::{
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn dispatch_textarea_key_action_emacs(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        if let Some(outcome) = self.dispatch_shared_textarea_key_action(action, count) {
            return outcome;
        }

        match action {
            CanvasKeyAction::DeleteSelection => {
                self.kill_region_emacs();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteSelectionNoYank => {
                self.delete_region_emacs();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::YankSelection => {
                self.copy_region_emacs();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteAfter => {
                self.paste_after_emacs(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_before_emacs(count);
                KeyEventOutcome::Consumed(None)
            }
            // Word/line kill commands. The implementations are paradigm-agnostic
            // (pure cursor/text edits); they back VSCode's Ctrl+Backspace /
            // Ctrl+Delete and Emacs-style line kills.
            CanvasKeyAction::DeleteWordBackward => {
                self.delete_word_backward_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteWordForward => {
                self.delete_word_forward_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteToLineStart => {
                self.delete_to_line_start_helix();
                KeyEventOutcome::Consumed(None)
            }
            _ => self.execute_canvas_key_action(action, count),
        }
    }
}
