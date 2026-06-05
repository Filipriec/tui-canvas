use crate::{
    canvas::state::SelectionState,
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn delete_selection_once(&mut self, yank: bool) -> bool {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current);
                let lines = self.editor.data_provider().capture_content();
                if start >= lines.len() {
                    return false;
                }

                if yank {
                    self.editor.behavior_state.yank_mut().set_line_register(
                        lines[start..=end.min(lines.len() - 1)]
                            .to_vec(),
                    );
                }

                self.editor
                    .record_checkpoint(crate::editor::features::history::EditKind::Delete);

                let mut content = lines;
                if content.len() <= 1 {
                    content = vec![String::new()];
                } else {
                    content.drain(start..=end.min(content.len() - 1));
                    if content.is_empty() {
                        content.push(String::new());
                    }
                }
                self.editor.data_provider_mut().restore_content(&content);
                let target = start.min(content.len().saturating_sub(1));
                let _ = self.transition_to_field(target);
                self.move_line_start();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                true
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                if anchor == cursor {
                    return self.delete_primary_character(yank);
                }

                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
                let lines = self.editor.data_provider().capture_content();
                if start.0 >= lines.len() || end.0 >= lines.len() {
                    return false;
                }

                if yank {
                    let yanked = self.extract_characterwise_text(&lines, start, end);
                    self.editor
                        .behavior_state
                        .yank_mut()
                        .set_text_register(yanked);
                }

                self.editor
                    .record_checkpoint(crate::editor::features::history::EditKind::Delete);

                let mut content = lines;
                if start.0 == end.0 {
                    let line = &content[start.0];
                    let new_line: String = line
                        .chars()
                        .enumerate()
                        .filter_map(|(idx, ch)| {
                            if idx < start.1 || idx > end.1 {
                                Some(ch)
                            } else {
                                None
                            }
                        })
                        .collect();
                    content[start.0] = new_line;
                } else {
                    let first: String = content[start.0]
                        .chars()
                        .take(start.1)
                        .collect();
                    let last: String = content[end.0].chars().skip(end.1 + 1).collect();
                    content[start.0] = format!("{first}{last}");
                    if end.0 > start.0 {
                        content.drain(start.0 + 1..=end.0);
                    }
                }

                self.editor.data_provider_mut().restore_content(&content);
                let _ = self.transition_to_field(start.0);
                self.set_cursor_position(start.1);
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                true
            }
            SelectionState::None => self.delete_primary_character(yank),
        }
    }

    pub(crate) fn delete_primary_character(&mut self, yank: bool) -> bool {
        let line_idx = self.current_field();
        let col = self.cursor_position();
        let current = self.current_text().to_string();
        let line_len = current.chars().count();

        if col < line_len {
            if yank {
                let ch: String = current.chars().skip(col).take(1).collect();
                self.editor
                    .behavior_state
                    .yank_mut()
                    .set_text_register(vec![ch]);
            }
            self.editor
                .record_checkpoint(crate::editor::features::history::EditKind::Delete);
            let kept: String = current
                .chars()
                .enumerate()
                .filter_map(|(idx, ch)| if idx == col { None } else { Some(ch) })
                .collect();
            self.editor
                .data_provider_mut()
                .set_field_value(line_idx, kept);
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            return true;
        }

        if line_idx + 1 < self.editor.data_provider().field_count() {
            if yank {
                let text = self
                    .editor
                    .data_provider()
                    .field_value(line_idx + 1)
                    .to_string();
                self.editor
                    .behavior_state
                    .yank_mut()
                    .set_text_register(vec![text]);
            }
            self.editor
                .record_checkpoint(crate::editor::features::history::EditKind::Delete);
            if let Some(new_col) = self.editor.data_provider_mut().join_with_next(line_idx) {
                self.set_cursor_position(new_col);
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                return true;
            }
        }

        false
    }

    pub(crate) fn extract_characterwise_text(
        &self,
        lines: &[String],
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<String> {
        if start.0 == end.0 {
            let text: String = lines[start.0]
                .chars()
                .skip(start.1)
                .take(end.1.saturating_sub(start.1) + 1)
                .collect();
            return vec![text];
        }

        let mut yanked = Vec::new();
        let first: String = lines[start.0].chars().skip(start.1).collect();
        yanked.push(first);
        for line in &lines[start.0 + 1..end.0] {
            yanked.push(line.clone());
        }
        let last: String = lines[end.0].chars().take(end.1 + 1).collect();
        yanked.push(last);
        yanked
    }
}
