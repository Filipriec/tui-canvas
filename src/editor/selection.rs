use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;
use crate::editor::behavior::KeybindingParadigm;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    pub(crate) fn uses_helix_paradigm(&self) -> bool {
        self.behavior_state.paradigm().is_helix()
    }

    pub(crate) fn ensure_helix_primary_selection(&mut self) {
        if self.ui_state.current_mode != AppMode::Nor {
            return;
        }
        let anchor = (self.ui_state.current_field, self.ui_state.cursor_pos);
        self.ui_state.selection = SelectionState::Characterwise { anchor };
    }

    pub(crate) fn collapse_selection_to_cursor(&mut self) {
        let anchor = (self.ui_state.current_field, self.ui_state.cursor_pos);
        self.ui_state.selection = SelectionState::Characterwise { anchor };
    }

    pub(crate) fn selection_endpoints(&self) -> ((usize, usize), (usize, usize)) {
        let cursor = (self.ui_state.current_field, self.ui_state.cursor_pos);
        match self.ui_state.selection {
            SelectionState::Characterwise { anchor } => (anchor.min(cursor), anchor.max(cursor)),
            SelectionState::Linewise { anchor_field } => {
                let start_field = anchor_field.min(self.ui_state.current_field);
                let end_field = anchor_field.max(self.ui_state.current_field);
                let end_line_len = self
                    .data_provider
                    .field_value(end_field)
                    .chars()
                    .count()
                    .saturating_sub(1);
                ((start_field, 0), (end_field, end_line_len))
            }
            SelectionState::None => (cursor, cursor),
        }
    }

    pub(crate) fn enter_insert_at_selection_start(&mut self) {
        let (start, _) = self.selection_endpoints();
        let _ = self.transition_to_field(start.0);
        self.set_cursor_position(start.1);
        self.ui_state.current_mode = AppMode::Ins;
        self.ui_state.selection = SelectionState::None;
        #[cfg(feature = "cursor-style")]
        {
            let _ = crate::cursor::CursorManager::update_for_mode(AppMode::Ins);
        }
        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();
    }

    pub(crate) fn enter_insert_after_selection(&mut self) {
        let (_, end) = self.selection_endpoints();
        let _ = self.transition_to_field(end.0);
        let line_len = self.current_text().chars().count();
        let append_pos = (end.1 + 1).min(line_len);
        self.set_cursor_position(append_pos);
        self.ui_state.current_mode = AppMode::Ins;
        self.ui_state.selection = SelectionState::None;
        #[cfg(feature = "cursor-style")]
        {
            let _ = crate::cursor::CursorManager::update_for_mode(AppMode::Ins);
        }
        #[cfg(feature = "suggestions")]
        self.check_suggestion_trigger();
    }

    pub(crate) fn apply_paradigm_after_mode_change(&mut self) {
        if self.behavior_state.paradigm() == KeybindingParadigm::Helix
            && self.ui_state.current_mode == AppMode::Nor
        {
            self.ensure_helix_primary_selection();
        } else if self.ui_state.current_mode == AppMode::Nor {
            self.ui_state.selection = SelectionState::None;
        }
    }
}
