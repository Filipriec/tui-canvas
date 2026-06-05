#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn dispatch_shared_textarea_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> Option<KeyEventOutcome> {
        match action {
            CanvasKeyAction::NextField if self.mode() == AppMode::Ins => {
                Some(self.consume_action(|| self.insert_tab_spaces()))
            }
            CanvasKeyAction::DeleteCharBackward => {
                Some(self.consume_action(|| {
                    for _ in 0..count {
                        self.delete_backward_preserving_mode();
                    }
                }))
            }
            CanvasKeyAction::DeleteCharForward => {
                Some(self.consume_action(|| {
                    for _ in 0..count {
                        self.delete_forward_preserving_mode();
                    }
                }))
            }
            CanvasKeyAction::OpenLineBelow => {
                Some(self.consume_action(|| {
                    for _ in 0..count {
                        self.open_line_below();
                    }
                }))
            }
            CanvasKeyAction::OpenLineAbove => {
                Some(self.consume_action(|| {
                    for _ in 0..count {
                        self.open_line_above();
                    }
                }))
            }
            CanvasKeyAction::MoveHalfPageUp => {
                Some(self.consume_action(|| self.move_half_page_up(count)))
            }
            CanvasKeyAction::MoveHalfPageDown => {
                Some(self.consume_action(|| self.move_half_page_down(count)))
            }
            CanvasKeyAction::EnterEditModeLineStart => {
                Some(self.consume_action(|| self.enter_line_start_insert_mode()))
            }
            CanvasKeyAction::EnterEditModeLineEnd => {
                Some(self.consume_action(|| self.enter_line_end_insert_mode()))
            }
            CanvasKeyAction::DeleteLine => {
                Some(self.consume_action(|| self.delete_current_lines(count)))
            }
            CanvasKeyAction::DeleteToLineEnd => {
                Some(self.consume_action(|| self.delete_to_line_end()))
            }
            CanvasKeyAction::ChangeLine => {
                Some(self.consume_action(|| self.change_current_line()))
            }
            CanvasKeyAction::ChangeToLineEnd => {
                Some(self.consume_action(|| self.change_to_line_end()))
            }
            CanvasKeyAction::JoinLineBelow => {
                Some(self.consume_action(|| self.join_lines_below(count)))
            }
            _ => None,
        }
    }

    pub(crate) fn consume_action(&mut self, f: impl FnOnce(&mut Self)) -> KeyEventOutcome {
        f(self);
        KeyEventOutcome::Consumed(None)
    }

    pub(crate) fn execute_canvas_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        let Some(canvas_action) = action.to_canvas_action() else {
            return KeyEventOutcome::NotMatched;
        };
        let mut result = crate::canvas::actions::ActionResult::Success;
        for _ in 0..count {
            result = self.editor.execute(canvas_action.clone());
        }
        match result {
            crate::canvas::actions::ActionResult::Success => KeyEventOutcome::Consumed(None),
            crate::canvas::actions::ActionResult::Message(msg)
            | crate::canvas::actions::ActionResult::Error(msg) => {
                KeyEventOutcome::Consumed(Some(msg))
            }
        }
    }
}
