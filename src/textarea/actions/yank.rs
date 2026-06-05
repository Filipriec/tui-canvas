use crate::{
    canvas::{modes::AppMode, state::SelectionState},
    editor::behavior::VimRegister,
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    #[cfg(feature = "keybindings")]
    pub fn yank_current_lines(&mut self, count: usize) {
        if self.mode() == AppMode::Sel {
            self.yank_selection();
            self.exit_highlight_mode();
            return;
        }

        let current = self.current_field();
        let lines = self.editor.data_provider().capture_content();
        if lines.is_empty() {
            self.editor
                .behavior_state
                .vim_mut()
                .set_line_register(vec![String::new()]);
            return;
        }

        let end = current.saturating_add(count.max(1)).min(lines.len());
        self.editor
            .behavior_state
            .vim_mut()
            .set_line_register(lines[current..end].to_vec());
    }

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
                        .vim_mut()
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
                    .vim_mut()
                    .set_line_register(yanked);
            }
            SelectionState::None => {}
        }
    }

    #[cfg(feature = "keybindings")]
    pub fn paste_after(&mut self, count: usize) {
        self.paste_yank(true, count);
    }

    #[cfg(feature = "keybindings")]
    pub fn paste_before(&mut self, count: usize) {
        self.paste_yank(false, count);
    }

    #[cfg(feature = "keybindings")]
    fn paste_yank(&mut self, after: bool, count: usize) {
        let Some(buffer) = self.editor.behavior_state.vim().register().cloned() else {
            return;
        };

        match buffer {
            VimRegister::Lines(lines) => {
                if lines.is_empty() {
                    return;
                }

                self.editor
                    .record_checkpoint(crate::editor::features::history::EditKind::Other);

                let repeat = count.max(1);
                let mut pasted = String::new();
                for i in 0..repeat {
                    if i > 0 {
                        pasted.push('\n');
                    }
                    pasted.push_str(&lines.join("\n"));
                }

                if lines.len() == 1 {
                    let field = self.current_field();
                    let line = self.editor.data_provider().field_value(field);
                    if lines[0] != line {
                        let (start, end) = self.selection_endpoints();
                        let insert_col = if after { end.1 + 1 } else { start.1 };
                        let mut chars: Vec<char> = line.chars().collect();
                        let byte_insert = insert_col.min(chars.len());
                        let insert_chars: Vec<char> = pasted.chars().collect();
                        chars.splice(byte_insert..byte_insert, insert_chars);
                        let new_line: String = chars.into_iter().collect();
                        self.editor
                            .data_provider_mut()
                            .set_field_value(field, new_line);
                        let _ = self.transition_to_field(field);
                        self.set_cursor_position(insert_col.saturating_add(pasted.chars().count()));
                        self.set_mode(AppMode::Nor);
                        #[cfg(feature = "keybindings")]
                        self.apply_paradigm_after_mode_change();
                        #[cfg(feature = "gui")]
                        {
                            self.edited_this_frame = true;
                        }
                        return;
                    }
                }

                let mut content = self.editor.data_provider().capture_content();
                let current = self.current_field().min(content.len().saturating_sub(1));
                let insert_at = if after {
                    current.saturating_add(1).min(content.len())
                } else {
                    current
                };

                let mut insert = Vec::with_capacity(lines.len() * repeat);
                for _ in 0..repeat {
                    insert.extend(lines.iter().cloned());
                }

                content.splice(insert_at..insert_at, insert);
                self.editor.data_provider_mut().restore_content(&content);
                let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
                self.move_line_start();
                self.set_mode(AppMode::Nor);
                #[cfg(feature = "keybindings")]
                self.apply_paradigm_after_mode_change();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
            }
        }
    }
}
