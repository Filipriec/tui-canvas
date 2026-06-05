#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn dispatch_textarea_key_action_vim(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        if let Some(outcome) = self.dispatch_shared_textarea_key_action(action, count) {
            return outcome;
        }

        match action {
            CanvasKeyAction::YankLine => {
                self.yank_current_lines_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteAfter => {
                self.paste_after_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_before_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::YankSelection => {
                for _ in 0..count {
                    self.yank_primary_selection_vim();
                }
                KeyEventOutcome::Consumed(None)
            }
            _ => self.execute_canvas_key_action(action, count),
        }
    }

    pub(crate) fn yank_current_lines_vim(&mut self, count: usize) {
        if self.mode() == AppMode::Sel {
            self.yank_primary_selection_vim();
            return;
        }

        let current = self.current_field();
        let lines = self.editor.data_provider().capture_content();
        if lines.is_empty() {
            self.editor
                .behavior_state
                .vim_mut()
                .set_line_yank_register(vec![String::new()]);
            return;
        }

        let end = current.saturating_add(count.max(1)).min(lines.len());
        self.editor
            .behavior_state
            .vim_mut()
            .set_line_yank_register(lines[current..end].to_vec());
    }
}
