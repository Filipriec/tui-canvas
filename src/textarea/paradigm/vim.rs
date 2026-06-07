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
            CanvasKeyAction::DeleteSelection => {
                self.delete_selection_vim(true, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteSelectionNoYank => {
                self.delete_selection_vim(false, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeSelection => {
                self.change_selection_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::SwitchCase => {
                if self.mode() == AppMode::Sel {
                    self.switch_case_selection_vim();
                } else {
                    self.toggle_case_char_vim(count);
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::IndentSelection => {
                self.indent_selection_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::UnindentSelection => {
                self.unindent_selection_vim(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::FindNextChar => {
                self.set_vim_pending(crate::textarea::actions::selection::vim::VimPending::Find {
                    till: false,
                    forward: true,
                    count,
                });
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::FindPrevChar => {
                self.set_vim_pending(crate::textarea::actions::selection::vim::VimPending::Find {
                    till: false,
                    forward: false,
                    count,
                });
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::TillNextChar => {
                self.set_vim_pending(crate::textarea::actions::selection::vim::VimPending::Find {
                    till: true,
                    forward: true,
                    count,
                });
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::TillPrevChar => {
                self.set_vim_pending(crate::textarea::actions::selection::vim::VimPending::Find {
                    till: true,
                    forward: false,
                    count,
                });
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ReplaceChar => {
                self.set_vim_pending(
                    crate::textarea::actions::selection::vim::VimPending::Replace { count },
                );
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::RepeatLastFind => {
                self.repeat_last_find_vim(false, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::RepeatLastFindReverse => {
                self.repeat_last_find_vim(true, count);
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
                .yank_mut()
                .set_line_register(vec![String::new()]);
            return;
        }

        let end = current.saturating_add(count.max(1)).min(lines.len());
        self.editor
            .behavior_state
            .yank_mut()
            .set_line_register(lines[current..end].to_vec());
    }
}
