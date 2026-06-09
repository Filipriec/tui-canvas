#[cfg(feature = "cursor-style")]
use crate::cursor::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::EditorCore;
use crate::DataProvider;

impl<D: DataProvider> EditorCore<D> {
    pub(crate) fn apply_after_mode_change_emacs(&mut self) {
        if self.ui_state.current_mode == AppMode::Nor {
            self.ui_state.selection = SelectionState::None;
        }
    }

    pub(crate) fn enter_edit_mode_emacs(&mut self) {
        self.set_mode_emacs(AppMode::Ins);
    }

    pub(crate) fn enter_append_mode_emacs(&mut self) {
        self.enter_edit_mode_emacs();
    }

    pub(crate) fn set_mode_emacs(&mut self, mode: AppMode) {
        if self.ui_state.current_mode != mode {
            self.break_undo_coalescing();
        }

        match (self.ui_state.current_mode, mode) {
            (AppMode::Nor, AppMode::Sel) => {
                self.enter_highlight_mode_emacs();
            }
            (AppMode::Sel, AppMode::Nor) => {
                self.exit_highlight_mode_emacs();
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
                self.apply_after_mode_change_emacs();
            }
        }
    }

    /// Emacs set-mark-command (`C-SPC`): anchor the region at point.
    pub(crate) fn enter_highlight_mode_emacs(&mut self) {
        match self.ui_state.current_mode {
            AppMode::Nor | AppMode::Sel => {
                self.set_highlight_mode_selection(SelectionState::Characterwise {
                    anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                });
            }
            _ => {}
        }
    }

    pub(crate) fn enter_highlight_line_mode_emacs(&mut self) {
        match self.ui_state.current_mode {
            AppMode::Nor | AppMode::Sel => {
                self.set_highlight_mode_selection(SelectionState::Linewise {
                    anchor_field: self.ui_state.current_field,
                });
            }
            _ => {}
        }
    }

    /// Emacs `C-g` / deactivate mark.
    pub(crate) fn exit_highlight_mode_emacs(&mut self) {
        if self.ui_state.current_mode == AppMode::Sel {
            self.ui_state.current_mode = AppMode::Nor;
            self.ui_state.selection = SelectionState::None;

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Nor);
            }
            self.apply_after_mode_change_emacs();
        }
    }
}
