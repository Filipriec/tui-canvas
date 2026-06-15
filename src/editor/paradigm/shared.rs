use crate::DataProvider;
use crate::canvas::state::SelectionState;
use crate::editor::EditorCore;

impl<D: DataProvider> EditorCore<D> {
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
}
