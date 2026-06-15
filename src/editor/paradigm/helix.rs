#[cfg(feature = "cursor-style")]
use crate::cursor::CursorManager;

use crate::DataProvider;
use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::EditorCore;

impl<D: DataProvider> EditorCore<D> {
    pub(crate) fn ensure_helix_primary_selection(&mut self) {
        if self.ui_state.current_mode != AppMode::Nor {
            return;
        }
        if !matches!(self.ui_state.selection, SelectionState::None) {
            return;
        }
        let anchor = (self.ui_state.current_field, self.ui_state.cursor_pos);
        self.ui_state.selection = SelectionState::Characterwise { anchor };
    }

    /// Collapse the selection to a single-character cursor selection, keeping the
    /// current mode. A paradigm-agnostic primitive (used by Helix `;`, `Esc`, and
    /// cursor movement) — not Helix-specific behavior.
    pub(crate) fn collapse_selection_to_cursor(&mut self) {
        let anchor = (self.ui_state.current_field, self.ui_state.cursor_pos);
        self.ui_state.selection = SelectionState::Characterwise { anchor };
    }

    pub(crate) fn apply_after_mode_change_helix(&mut self) {
        if self.ui_state.current_mode == AppMode::Nor {
            self.ensure_helix_primary_selection();
        }
    }

    pub(crate) fn enter_edit_mode_helix(&mut self) {
        self.enter_insert_at_selection_start_helix();
    }

    pub(crate) fn enter_append_mode_helix(&mut self) {
        self.enter_insert_after_selection_helix();
    }

    pub(crate) fn enter_insert_at_selection_start_helix(&mut self) {
        let (start, _) = self.selection_endpoints();
        let _ = self.transition_to_field(start.0);
        self.set_cursor_position(start.1);
        self.ui_state.current_mode = AppMode::Ins;
        self.ui_state.selection = SelectionState::None;
        #[cfg(feature = "cursor-style")]
        {
            let _ = CursorManager::update_for_mode(AppMode::Ins);
        }
        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();
    }

    pub(crate) fn enter_insert_after_selection_helix(&mut self) {
        let (_, end) = self.selection_endpoints();
        let _ = self.transition_to_field(end.0);
        let line_len = self.current_text().chars().count();
        let append_pos = (end.1 + 1).min(line_len);
        // Switch to insert mode *before* positioning the cursor: in normal mode
        // `set_cursor_position` clamps to `len - 1`, which would snap the append
        // position back onto the last character instead of past it.
        self.ui_state.current_mode = AppMode::Ins;
        self.set_cursor_position(append_pos);
        self.ui_state.selection = SelectionState::None;
        #[cfg(feature = "cursor-style")]
        {
            let _ = CursorManager::update_for_mode(AppMode::Ins);
        }
        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();
    }

    pub(crate) fn set_mode_helix(&mut self, mode: AppMode) {
        if self.ui_state.current_mode != mode {
            self.break_undo_coalescing();
        }

        match (self.ui_state.current_mode, mode) {
            (AppMode::Nor, AppMode::Sel) => {
                self.enter_highlight_mode_helix();
            }
            (AppMode::Sel, AppMode::Nor) => {
                self.exit_highlight_mode_helix();
            }
            (_, new_mode) => {
                self.ui_state.current_mode = new_mode;
                if new_mode != AppMode::Sel {
                    self.ui_state.selection = SelectionState::None;
                }
                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(new_mode);
                }
                self.apply_after_mode_change_helix();
            }
        }
    }

    pub(crate) fn enter_highlight_mode_helix(&mut self) {
        match (&self.ui_state.current_mode, &self.ui_state.selection) {
            (AppMode::Nor, _) => {
                let anchor = match self.ui_state.selection {
                    SelectionState::Characterwise { anchor } => anchor,
                    _ => (self.ui_state.current_field, self.ui_state.cursor_pos),
                };
                self.set_highlight_mode_selection(SelectionState::Characterwise { anchor });
            }
            (AppMode::Sel, SelectionState::Characterwise { .. }) => {
                self.exit_highlight_mode_helix();
            }
            (AppMode::Sel, _) => {
                self.set_highlight_mode_selection(SelectionState::Characterwise {
                    anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                });
            }
            _ => {}
        }
    }

    pub(crate) fn enter_highlight_line_mode_helix(&mut self) {
        match (&self.ui_state.current_mode, &self.ui_state.selection) {
            (AppMode::Nor, _) => {
                self.set_highlight_mode_selection(SelectionState::Linewise {
                    anchor_field: self.ui_state.current_field,
                });
            }
            (AppMode::Sel, SelectionState::Linewise { .. }) => {
                self.exit_highlight_mode_helix();
            }
            (AppMode::Sel, SelectionState::Characterwise { anchor }) => {
                self.set_highlight_mode_selection(SelectionState::Linewise {
                    anchor_field: anchor.0,
                });
            }
            (AppMode::Sel, _) => {
                self.set_highlight_mode_selection(SelectionState::Linewise {
                    anchor_field: self.ui_state.current_field,
                });
            }
            _ => {}
        }
    }

    pub(crate) fn exit_highlight_mode_helix(&mut self) {
        if self.ui_state.current_mode == AppMode::Sel {
            self.ui_state.current_mode = AppMode::Nor;

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Nor);
            }
            self.apply_after_mode_change_helix();
        }
    }

    /// Yank the primary selection, then leave select mode (Helix `y`). Shared by
    /// the text area and the text form so both collapse the selection on yank.
    pub(crate) fn yank_primary_selection_helix(&mut self) {
        self.yank_selection_core();
        if self.ui_state.current_mode == AppMode::Sel {
            self.exit_highlight_mode_helix();
        }
    }

    /// Helix `x`: snap the selection to whole lines, growing one line downward
    /// on repeat. Stays in normal mode with a linewise selection (Helix has no
    /// separate visual mode for this) — shared so forms and areas behave alike.
    pub(crate) fn extend_line_below_helix(&mut self) {
        let current = self.current_field();
        let field_count = self.data_provider().field_count();
        self.ui_state.current_mode = AppMode::Nor;

        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let next = (current + 1).min(field_count.saturating_sub(1));
                if next != current {
                    let _ = self.transition_to_field(next);
                }
                self.ui_state.selection = SelectionState::Linewise { anchor_field };
            }
            SelectionState::Characterwise { anchor } => {
                self.ui_state.selection = SelectionState::Linewise {
                    anchor_field: anchor.0,
                };
            }
            SelectionState::None => {
                self.ui_state.selection = SelectionState::Linewise {
                    anchor_field: current,
                };
            }
        }
    }

    /// Helix `X`: snap the selection to whole lines without moving to the next
    /// line, preserving the anchor line.
    pub(crate) fn extend_to_line_bounds_helix(&mut self) {
        let current = self.current_field();
        self.ui_state.current_mode = AppMode::Nor;

        let anchor_field = match self.selection_state() {
            SelectionState::Linewise { anchor_field } => *anchor_field,
            SelectionState::Characterwise { anchor } => anchor.0,
            SelectionState::None => current,
        };
        self.ui_state.selection = SelectionState::Linewise { anchor_field };
    }

    /// Return to normal mode after a Helix selection edit (`d`). If the edit ran
    /// from highlight mode (`v`), leave it; then re-establish the primary cursor
    /// selection. Shared so the text area and the text form behave identically.
    pub(crate) fn finish_helix_selection_edit(&mut self) {
        if self.ui_state.current_mode == AppMode::Sel {
            self.exit_highlight_mode_helix();
        }
        if self.ui_state.current_mode == AppMode::Nor {
            self.ensure_helix_primary_selection();
        }
    }
}
