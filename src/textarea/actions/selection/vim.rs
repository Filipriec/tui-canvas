use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::textarea::actions::selection::helix::HelixCase;
use crate::textarea::{TextAreaDataProvider, TextAreaState};

/// A Vim command waiting for the next literal character (`f`, `t`, `r`, …).
///
/// Vim resolves these one keystroke at a time exactly like Helix, but the
/// motions are cursor-only (no persistent selection in normal mode), so the
/// resolved handlers are the `*_vim` variants rather than the Helix ones.
#[derive(Clone, Copy)]
pub(crate) enum VimPending {
    /// `f`/`F`/`t`/`T`: move to (or up to) the `count`-th typed char on the line.
    Find {
        till: bool,
        forward: bool,
        count: usize,
    },
    /// `r`: replace `count` characters under the cursor with the typed char.
    Replace { count: usize },
}

/// The most recent find, so `;`/`,` can repeat it.
#[derive(Clone, Copy)]
pub(crate) struct VimFind {
    pub ch: char,
    pub till: bool,
    pub forward: bool,
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    fn field_chars_vim(&self, field: usize) -> Vec<char> {
        self.core
            .data_provider()
            .field_value(field)
            .chars()
            .collect()
    }

    pub(crate) fn yank_primary_selection_vim(&mut self) {
        self.yank_selection();
        self.exit_highlight_mode_vim();
    }

    /// Visual-mode `d`/`x`: delete the selection (optionally yanking) and drop
    /// back to normal mode. Reuses the shared selection-delete primitive.
    pub(crate) fn delete_selection_vim(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        self.exit_highlight_mode_vim();
    }

    /// Visual-mode `c`/`s`: delete the selection and enter insert mode at the
    /// start of the removed text.
    pub(crate) fn change_selection_vim(&mut self, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(true) {
                break;
            }
        }
        self.enter_edit_mode_vim();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// Visual-mode `~`: toggle the case of the selection, then return to normal.
    pub(crate) fn switch_case_selection_vim(&mut self) {
        self.switch_case_selection_helix(HelixCase::Toggle);
        self.exit_highlight_mode_vim();
    }

    /// Visual-mode `>`: indent the selected lines once and return to normal.
    pub(crate) fn indent_selection_vim(&mut self, count: usize) {
        self.indent_selection_helix(count);
        self.exit_highlight_mode_vim();
    }

    /// Visual-mode `<`: unindent the selected lines once and return to normal.
    pub(crate) fn unindent_selection_vim(&mut self, count: usize) {
        self.unindent_selection_helix(count);
        self.exit_highlight_mode_vim();
    }

    /// Normal-mode `~`: toggle the case of `count` characters under the cursor,
    /// advancing the cursor past them. Reuses the Helix case mapper over a
    /// transient one-shot selection.
    pub(crate) fn toggle_case_char_vim(&mut self, count: usize) {
        let field = self.current_field();
        let len = self.field_chars_vim(field).len();
        if len == 0 {
            return;
        }
        let cursor = self.cursor_position();
        let end = (cursor + count.max(1) - 1).min(len - 1);

        self.core.ui_state.selection = SelectionState::Characterwise {
            anchor: (field, cursor),
        };
        self.core.ui_state.set_cursor(end, len, false);
        self.switch_case_selection_helix(HelixCase::Toggle);

        let next = (end + 1).min(len.saturating_sub(1));
        self.core.ui_state.set_cursor(next, len, false);
        self.core.ui_state.selection = SelectionState::None;
    }

    /// `r<char>`: replace `count` characters under the cursor with `ch`. Fails
    /// (no-op) when the line has fewer than `count` characters remaining, just
    /// like Vim. Reuses the Helix selection replace over a transient selection.
    pub(crate) fn replace_char_vim(&mut self, ch: char, count: usize) {
        let field = self.current_field();
        let len = self.field_chars_vim(field).len();
        let cursor = self.cursor_position();
        let count = count.max(1);
        if len == 0 || cursor + count > len {
            return;
        }
        let end = cursor + count - 1;

        self.core.ui_state.selection = SelectionState::Characterwise {
            anchor: (field, cursor),
        };
        self.core.ui_state.set_cursor(end, len, false);
        self.replace_selection_with_char_helix(ch);

        // Vim leaves the cursor on the last replaced character.
        self.core.ui_state.set_cursor(end, len, false);
        self.core.ui_state.selection = SelectionState::None;
    }

    /// `f`/`F`/`t`/`T`: move the cursor to the `count`-th occurrence of `ch` on
    /// the current line. In normal mode the cursor simply moves; in visual mode
    /// the selection head is extended to the target.
    pub(crate) fn find_char_vim(&mut self, ch: char, till: bool, forward: bool, count: usize) {
        let field = self.current_field();
        let line = self.field_chars_vim(field);
        let len = line.len();
        let cursor = self.cursor_position();
        let count = count.max(1);

        let mut pos = cursor;
        if forward {
            for _ in 0..count {
                match ((pos + 1)..len).find(|&j| line[j] == ch) {
                    Some(hit) => pos = hit,
                    None => return,
                }
            }
            if till {
                if pos == cursor + 1 && count == 1 {
                    // Nothing to land on between the cursor and the match.
                    return;
                }
                pos = pos.saturating_sub(1);
            }
        } else {
            for _ in 0..count {
                if pos == 0 {
                    return;
                }
                match (0..pos).rev().find(|&j| line[j] == ch) {
                    Some(hit) => pos = hit,
                    None => return,
                }
            }
            if till {
                pos = (pos + 1).min(len.saturating_sub(1));
            }
        }
        if pos == cursor {
            return;
        }

        if self.mode() == AppMode::Sel {
            let anchor = match self.selection_state() {
                SelectionState::Characterwise { anchor } => *anchor,
                _ => (field, cursor),
            };
            self.core.ui_state.set_cursor(pos, len, false);
            self.core.ui_state.selection = SelectionState::Characterwise { anchor };
        } else {
            self.core.ui_state.set_cursor(pos, len, false);
            self.core.ui_state.selection = SelectionState::None;
        }
    }

    /// `;` (and `,` with `reverse`): repeat the last find/till motion.
    pub(crate) fn repeat_last_find_vim(&mut self, reverse: bool, count: usize) {
        if let Some(find) = self.vim_last_find {
            let forward = if reverse { !find.forward } else { find.forward };
            self.find_char_vim(find.ch, find.till, forward, count);
        }
    }

    pub(crate) fn set_vim_pending(&mut self, pending: VimPending) {
        self.vim_pending = Some(pending);
    }

    /// Resolve a pending Vim command with the character the user just typed.
    pub(crate) fn resolve_vim_pending(&mut self, pending: VimPending, ch: char) {
        match pending {
            VimPending::Find {
                till,
                forward,
                count,
            } => {
                self.vim_last_find = Some(VimFind { ch, till, forward });
                self.find_char_vim(ch, till, forward, count);
            }
            VimPending::Replace { count } => self.replace_char_vim(ch, count),
        }
    }
}
