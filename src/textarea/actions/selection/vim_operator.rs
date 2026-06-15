#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    canvas::state::SelectionState,
    editor::behavior::{VimOperator, VimPendingOperator},
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::actions::selection::vim::VimPending,
    textarea::{TextAreaDataProvider, TextAreaState},
};

/// How a motion delimits the text an operator acts on.
#[cfg(feature = "keybindings")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum MotionKind {
    /// Whole-line operation (`dd`, `dj`, `dG`).
    Linewise,
    /// Character range excluding the character at the far end (`dw`, `db`, `d0`).
    CharExclusive,
    /// Character range including the character at the far end (`de`, `d$`, `df`).
    CharInclusive,
    /// Not a valid operator motion — cancels the pending operator.
    Unsupported,
}

#[cfg(feature = "keybindings")]
fn motion_kind(action: &CanvasKeyAction) -> MotionKind {
    use CanvasKeyAction::*;
    match action {
        MoveWordNext
        | MoveBigWordNext
        | MoveWordPrev
        | MoveBigWordPrev
        | MoveLeft
        | MoveRight
        | MoveLineStart
        | GotoFirstNonWhitespace => MotionKind::CharExclusive,
        MoveWordEnd | MoveBigWordEnd | MoveWordEndPrev | MoveBigWordEndPrev | MoveLineEnd => {
            MotionKind::CharInclusive
        }
        MoveUp | MoveDown | MoveFirstLine | MoveLastLine => MotionKind::Linewise,
        _ => MotionKind::Unsupported,
    }
}

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// `d`/`c`/`y` in normal mode: capture the operator and wait for a motion.
    pub(crate) fn begin_operator_vim(&mut self, operator: VimOperator, count: usize) {
        let anchor = (self.current_field(), self.cursor_position());
        self.core
            .behavior_state
            .vim_mut()
            .set_pending_operator(VimPendingOperator {
                operator,
                count: count.max(1),
                anchor,
            });
    }

    fn field_len_vim(&self, field: usize) -> usize {
        self.core.data_provider().field_value(field).chars().count()
    }

    /// The position one character before `pos`, crossing into the previous
    /// field's last character when at a line start. `None` at the very start.
    fn dec_position_vim(&self, pos: (usize, usize)) -> Option<(usize, usize)> {
        if pos.1 > 0 {
            Some((pos.0, pos.1 - 1))
        } else if pos.0 > 0 {
            Some((pos.0 - 1, self.field_len_vim(pos.0 - 1).saturating_sub(1)))
        } else {
            None
        }
    }

    /// A motion key arrived while an operator was pending. Resolve the operated
    /// range and apply the operator, or cancel on an invalid key.
    pub(crate) fn apply_operator_motion_vim(
        &mut self,
        action: &CanvasKeyAction,
        motion_count: usize,
    ) -> KeyEventOutcome {
        let Some(pending) = self.core.behavior_state.vim().pending_operator() else {
            return self.execute_canvas_key_action(action, motion_count);
        };
        let total = pending.count.saturating_mul(motion_count.max(1)).max(1);

        // Doubled operator (`dd`, `cc`, `yy`) → linewise on the current line(s).
        if matches!(
            action,
            CanvasKeyAction::OperatorDelete
                | CanvasKeyAction::OperatorChange
                | CanvasKeyAction::OperatorYank
        ) {
            self.core.behavior_state.vim_mut().clear_pending_operator();
            let start = pending.anchor.0;
            self.apply_operator_linewise_vim(
                pending.operator,
                start,
                start.saturating_add(total).saturating_sub(1),
            );
            return KeyEventOutcome::Consumed(None);
        }

        // `f`/`t`/`F`/`T` need a character: re-arm the pending char and keep the
        // operator armed. The capture in `handle_key_event` finishes the operator
        // once the character moves the cursor.
        let pending_find = match action {
            CanvasKeyAction::FindNextChar => Some((false, true)),
            CanvasKeyAction::FindPrevChar => Some((false, false)),
            CanvasKeyAction::TillNextChar => Some((true, true)),
            CanvasKeyAction::TillPrevChar => Some((true, false)),
            _ => None,
        };
        if let Some((till, forward)) = pending_find {
            self.set_vim_pending(VimPending::Find {
                till,
                forward,
                count: total,
            });
            return KeyEventOutcome::Consumed(None);
        }

        if matches!(
            action,
            CanvasKeyAction::RepeatLastFind | CanvasKeyAction::RepeatLastFindReverse
        ) {
            self.core.behavior_state.vim_mut().clear_pending_operator();
            if let Some(find) = self.vim_last_find {
                let reverse = matches!(action, CanvasKeyAction::RepeatLastFindReverse);
                let forward = if reverse { !find.forward } else { find.forward };
                self.repeat_last_find_vim(reverse, total);
                self.finish_operator_charwise_vim(pending.operator, pending.anchor, forward);
            }
            return KeyEventOutcome::Consumed(None);
        }

        // `cw`/`cW` act like `ce`/`cE` (don't swallow the trailing whitespace).
        let resolved = if pending.operator == VimOperator::Change {
            match action {
                CanvasKeyAction::MoveWordNext => CanvasKeyAction::MoveWordEnd,
                CanvasKeyAction::MoveBigWordNext => CanvasKeyAction::MoveBigWordEnd,
                other => other.clone(),
            }
        } else {
            action.clone()
        };

        match motion_kind(&resolved) {
            MotionKind::Linewise => {
                self.core.behavior_state.vim_mut().clear_pending_operator();
                let start_field = pending.anchor.0;
                let _ = self.execute_canvas_key_action(&resolved, total);
                let end_field = self.current_field();
                // `j`/`k` are relative: if the cursor couldn't move (already at the
                // top/bottom), the operator aborts — Vim does not touch the line.
                // `gg`/`G` are absolute, so landing on the same line is a real
                // single-line range.
                let relative = matches!(
                    resolved,
                    CanvasKeyAction::MoveDown | CanvasKeyAction::MoveUp
                );
                if relative && end_field == start_field {
                    return KeyEventOutcome::Consumed(None);
                }
                self.apply_operator_linewise_vim(
                    pending.operator,
                    start_field.min(end_field),
                    start_field.max(end_field),
                );
                KeyEventOutcome::Consumed(None)
            }
            MotionKind::CharExclusive | MotionKind::CharInclusive => {
                let mut inclusive = motion_kind(&resolved) == MotionKind::CharInclusive;
                let forward_word = matches!(
                    resolved,
                    CanvasKeyAction::MoveWordNext | CanvasKeyAction::MoveBigWordNext
                );

                // Run the motion one step at a time so we can tell when it can no
                // longer advance (e.g. `w` on the final word).
                let mut stalled = false;
                for _ in 0..total {
                    let before = (self.current_field(), self.cursor_position());
                    let _ = self.execute_canvas_key_action(&resolved, 1);
                    if (self.current_field(), self.cursor_position()) == before {
                        stalled = true;
                        break;
                    }
                }

                // `dw`/`dW` that runs out of words still deletes through the end
                // of the current line in Vim.
                if forward_word && stalled {
                    let field = self.current_field();
                    let len = self.field_len_vim(field);
                    if len > 0 {
                        self.core.ui_state.set_cursor(len - 1, len, false);
                        inclusive = true;
                    }
                }

                self.core.behavior_state.vim_mut().clear_pending_operator();
                self.finish_operator_charwise_vim(pending.operator, pending.anchor, inclusive);
                KeyEventOutcome::Consumed(None)
            }
            MotionKind::Unsupported => {
                self.core.behavior_state.vim_mut().clear_pending_operator();
                KeyEventOutcome::Consumed(None)
            }
        }
    }

    /// Apply the operator to the characterwise span between `anchor` and the
    /// current cursor. `inclusive` controls whether the character at the far end
    /// is part of the range.
    pub(crate) fn finish_operator_charwise_vim(
        &mut self,
        operator: VimOperator,
        anchor: (usize, usize),
        inclusive: bool,
    ) {
        let cursor = (self.current_field(), self.cursor_position());
        if cursor == anchor {
            return; // the motion didn't move — nothing to operate on
        }

        let (lo, mut hi) = if anchor <= cursor {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        };
        if !inclusive {
            match self.dec_position_vim(hi) {
                Some(p) if p >= lo => hi = p,
                _ => return, // exclusive range collapsed to nothing
            }
        }

        self.core.ui_state.selection = SelectionState::Characterwise { anchor: lo };
        let _ = self.transition_to_field(hi.0);
        let len = self.field_len_vim(hi.0);
        self.core.ui_state.set_cursor(hi.1, len, false);

        match operator {
            VimOperator::Delete => {
                self.delete_selection_once(true);
                self.core.ui_state.selection = SelectionState::None;
            }
            VimOperator::Yank => {
                self.yank_selection();
                let _ = self.transition_to_field(lo.0);
                self.set_cursor_position(lo.1);
                self.core.ui_state.selection = SelectionState::None;
            }
            VimOperator::Change => {
                self.delete_selection_once(true);
                self.core.ui_state.selection = SelectionState::None;
                self.enter_edit_mode_vim();
            }
        }
    }

    /// Apply the operator to the whole-line range `[start_field, end_field]`.
    fn apply_operator_linewise_vim(
        &mut self,
        operator: VimOperator,
        start_field: usize,
        end_field: usize,
    ) {
        let last = self.core.data_provider().field_count().saturating_sub(1);
        let start = start_field.min(last);
        let end = end_field.min(last);
        let count = end - start + 1;

        let _ = self.transition_to_field(start);
        match operator {
            VimOperator::Delete => {
                // Reuse the linewise selection delete so the lines land in the
                // yank register (Vim's `dd` is also a yank).
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: start,
                };
                let _ = self.transition_to_field(end);
                self.delete_selection_once(true);
                self.core.ui_state.selection = SelectionState::None;
            }
            VimOperator::Yank => {
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: start,
                };
                let _ = self.transition_to_field(end);
                self.yank_selection();
                let _ = self.transition_to_field(start);
                self.move_line_start();
                self.core.ui_state.selection = SelectionState::None;
                if self.mode() != AppMode::Nor {
                    self.set_mode_vim(AppMode::Nor);
                }
            }
            VimOperator::Change => {
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: start,
                };
                let _ = self.transition_to_field(end);
                self.yank_selection();
                self.core.ui_state.selection = SelectionState::None;
                let _ = self.transition_to_field(start);

                if count <= 1 {
                    self.change_current_line();
                } else {
                    let _ = self.transition_to_field(start + 1);
                    self.delete_current_lines(count - 1);
                    let _ = self.transition_to_field(start);
                    self.change_current_line();
                }
            }
        }
    }
}
