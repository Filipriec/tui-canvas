#[cfg(feature = "cursor-style")]
use crate::cursor::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::EditorCore;
use crate::DataProvider;

impl<D: DataProvider> EditorCore<D> {
    pub(crate) fn apply_after_mode_change_vim(&mut self) {
        if self.ui_state.current_mode == AppMode::Nor {
            self.ui_state.selection = SelectionState::None;
        }
    }

    pub(crate) fn enter_edit_mode_vim(&mut self) {
        self.set_mode_vim(AppMode::Ins);
    }

    pub(crate) fn enter_append_mode_vim(&mut self) {
        let current_text = self.current_text();
        let char_len = current_text.chars().count();
        let append_pos = if current_text.is_empty() {
            0
        } else {
            (self.ui_state.cursor_pos + 1).min(char_len)
        };

        self.set_cursor_raw(append_pos);
        self.set_mode_vim(AppMode::Ins);
    }

    pub(crate) fn set_mode_vim(&mut self, mode: AppMode) {
        if self.ui_state.current_mode != mode {
            self.break_undo_coalescing();
        }

        match (self.ui_state.current_mode, mode) {
            (AppMode::Nor, AppMode::Sel) => {
                self.enter_highlight_mode_vim();
            }
            (AppMode::Sel, AppMode::Nor) => {
                self.exit_highlight_mode_vim();
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
                self.apply_after_mode_change_vim();
            }
        }
    }

    pub(crate) fn enter_highlight_mode_vim(&mut self) {
        match (&self.ui_state.current_mode, &self.ui_state.selection) {
            (AppMode::Nor, _) => {
                self.set_highlight_mode_selection(SelectionState::Characterwise {
                    anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                });
            }
            (AppMode::Sel, SelectionState::Characterwise { .. }) => {
                self.exit_highlight_mode_vim();
            }
            (AppMode::Sel, _) => {
                self.set_highlight_mode_selection(SelectionState::Characterwise {
                    anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
                });
            }
            _ => {}
        }
    }

    pub(crate) fn enter_highlight_line_mode_vim(&mut self) {
        match (&self.ui_state.current_mode, &self.ui_state.selection) {
            (AppMode::Nor, _) => {
                self.set_highlight_mode_selection(SelectionState::Linewise {
                    anchor_field: self.ui_state.current_field,
                });
            }
            (AppMode::Sel, SelectionState::Linewise { .. }) => {
                self.exit_highlight_mode_vim();
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

    pub(crate) fn exit_highlight_mode_vim(&mut self) {
        if self.ui_state.current_mode == AppMode::Sel {
            self.ui_state.current_mode = AppMode::Nor;
            self.ui_state.selection = SelectionState::None;

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Nor);
            }
            self.apply_after_mode_change_vim();
        }
    }
}
