use crate::{
    canvas::{modes::AppMode, state::SelectionState},
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn delete_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        if self.mode() == AppMode::Nor {
            self.ensure_helix_primary_selection();
        }
    }

    pub(crate) fn change_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        self.enter_edit_mode_helix();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    pub(crate) fn yank_primary_selection_helix(&mut self) {
        self.yank_selection();
        if self.mode() == AppMode::Sel {
            self.exit_highlight_mode_helix();
        }
    }

    pub(crate) fn collapse_selection_helix(&mut self) {
        self.collapse_helix_selection_to_cursor();
    }

    pub(crate) fn extend_line_below_helix(&mut self) {
        let current = self.current_field();
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } if anchor_field == current => {
                let next = current.saturating_add(1);
                if next < self.editor.data_provider().field_count() {
                    let _ = self.transition_to_field(next);
                    self.ui_state.current_mode = AppMode::Nor;
                    self.ui_state.selection = SelectionState::Linewise { anchor_field };
                }
            }
            _ => {
                self.ui_state.current_mode = AppMode::Nor;
                self.ui_state.selection = SelectionState::Linewise { anchor_field: current };
            }
        }
    }

    pub(crate) fn extend_to_line_bounds_helix(&mut self) {
        let current = self.current_field();
        self.ui_state.current_mode = AppMode::Nor;
        self.ui_state.selection = SelectionState::Linewise { anchor_field: current };
    }
}
