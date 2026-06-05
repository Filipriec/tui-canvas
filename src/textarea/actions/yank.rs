use crate::{
    canvas::state::SelectionState,
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    #[cfg(feature = "keybindings")]
    pub(crate) fn yank_selection(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current);
                let lines = self.editor.data_provider().capture_content();
                if start < lines.len() {
                    self.editor
                        .behavior_state
                        .yank_mut()
                        .set_line_register(lines[start..=end.min(lines.len() - 1)].to_vec());
                }
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
                let lines = self.editor.data_provider().capture_content();
                if start.0 >= lines.len() || end.0 >= lines.len() {
                    return;
                }

                let mut yanked = Vec::new();
                if start.0 == end.0 {
                    if start.1 == end.1 {
                        let text: String = lines[start.0].chars().skip(start.1).take(1).collect();
                        if text.is_empty() {
                            return;
                        }
                        yanked.push(text);
                    } else {
                        let text: String = lines[start.0]
                            .chars()
                            .skip(start.1)
                            .take(end.1.saturating_sub(start.1) + 1)
                            .collect();
                        yanked.push(text);
                    }
                } else {
                    let first: String = lines[start.0].chars().skip(start.1).collect();
                    yanked.push(first);
                    for line in &lines[start.0 + 1..end.0] {
                        yanked.push(line.clone());
                    }
                    let last: String = lines[end.0].chars().take(end.1 + 1).collect();
                    yanked.push(last);
                }
                self.editor
                    .behavior_state
                    .yank_mut()
                    .set_text_register(yanked);
            }
            SelectionState::None => {}
        }
    }
}
