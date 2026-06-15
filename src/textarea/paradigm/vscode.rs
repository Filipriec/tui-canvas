#[cfg(feature = "keybindings")]
use crate::{
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Modeless, VSCode-style dispatch.
    ///
    /// Layered on top of the non-modal Emacs dispatch: this adds Shift+movement
    /// selection extension, makes plain movement collapse a selection, and makes
    /// delete/paste/copy/cut respect an active selection. Anything else falls
    /// through to the Emacs path (movement, word kills, undo/redo, line ops, …).
    pub(crate) fn dispatch_textarea_key_action_vscode(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        use CanvasKeyAction::*;

        match action {
            // --- Selection extension (Shift+movement) ---
            SelectLeft => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        let _ = s.move_left();
                    }
                })
            }),
            SelectRight => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        let _ = s.move_right();
                    }
                })
            }),
            SelectUp => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        let _ = s.move_up();
                    }
                })
            }),
            SelectDown => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        let _ = s.move_down();
                    }
                })
            }),
            SelectWordPrev => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        s.move_word_prev();
                    }
                })
            }),
            SelectWordNext => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    for _ in 0..count {
                        s.move_word_next();
                    }
                })
            }),
            SelectLineStart => self.consume_action(|t| t.vscode_extend(|s| s.move_line_start())),
            SelectLineEnd => self.consume_action(|t| t.vscode_extend(|s| s.move_line_end())),
            SelectDocStart => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    let _ = s.move_first_line();
                })
            }),
            SelectDocEnd => self.consume_action(|t| {
                t.vscode_extend(|s| {
                    let _ = s.move_last_line();
                })
            }),
            SelectAll => self.consume_action(|t| t.vscode_select_all()),

            // --- Copy / cut respect an active selection, else act on the line ---
            CopyLine => {
                if self.vscode_selection_active() {
                    let _ = self.vscode_copy_selection();
                } else {
                    self.copy_current_line();
                }
                KeyEventOutcome::Consumed(None)
            }
            CutLine => {
                if self.vscode_selection_active() {
                    let _ = self.vscode_copy_selection();
                    self.vscode_delete_selection();
                } else {
                    self.cut_current_line();
                }
                KeyEventOutcome::Consumed(None)
            }

            // --- Backspace/Delete over a selection deletes the selection ---
            DeleteCharBackward | DeleteCharForward if self.vscode_selection_active() => {
                self.vscode_delete_selection();
                KeyEventOutcome::Consumed(None)
            }

            // --- Paste over a selection replaces it ---
            PasteAfter | PasteBefore if self.vscode_selection_active() => {
                self.vscode_delete_selection();
                self.dispatch_textarea_key_action_emacs(action, count)
            }

            // --- Plain caret movement collapses any active selection ---
            MoveLeft | MoveRight | MoveUp | MoveDown | MoveWordNext | MoveWordPrev
            | MoveWordEnd | MoveWordEndPrev | MoveBigWordNext | MoveBigWordPrev
            | MoveBigWordEnd | MoveBigWordEndPrev | MoveLineStart | MoveLineEnd | MoveFirstLine
            | MoveLastLine | MovePageUp | MovePageDown | MoveHalfPageUp | MoveHalfPageDown
                if self.vscode_selection_active() =>
            {
                self.vscode_clear_selection();
                self.dispatch_textarea_key_action_emacs(action, count)
            }

            _ => self.dispatch_textarea_key_action_emacs(action, count),
        }
    }
}
