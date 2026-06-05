#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    editor::behavior::VimRegister,
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn paste_after_vim(&mut self, count: usize) {
        self.paste_yank_vim(true, count);
    }

    pub(crate) fn paste_before_vim(&mut self, count: usize) {
        self.paste_yank_vim(false, count);
    }

    pub(crate) fn paste_after_helix(&mut self, count: usize) {
        self.paste_yank_helix(true, count);
    }

    pub(crate) fn paste_before_helix(&mut self, count: usize) {
        self.paste_yank_helix(false, count);
    }

    fn paste_yank_vim(&mut self, after: bool, count: usize) {
        let Some(VimRegister::Lines(lines)) = self.editor.behavior_state.vim().register().cloned()
        else {
            return;
        };
        if lines.is_empty() {
            return;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut content = self.editor.data_provider().capture_content();
        let current = self.current_field().min(content.len().saturating_sub(1));
        let insert_at = if after {
            current.saturating_add(1).min(content.len())
        } else {
            current
        };

        let repeat = count.max(1);
        let mut insert = Vec::with_capacity(lines.len() * repeat);
        for _ in 0..repeat {
            insert.extend(lines.iter().cloned());
        }

        content.splice(insert_at..insert_at, insert);
        self.editor.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
        self.move_line_start();
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn paste_yank_helix(&mut self, after: bool, count: usize) {
        let Some(VimRegister::Lines(lines)) = self.editor.behavior_state.vim().register().cloned()
        else {
            return;
        };
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
                self.ui_state.current_mode = AppMode::Nor;
                self.ensure_helix_primary_selection();
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
        self.ui_state.current_mode = AppMode::Nor;
        self.ensure_helix_primary_selection();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }
}
