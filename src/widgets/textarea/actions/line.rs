use crate::{
    canvas::modes::AppMode,
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn enter_line_start_insert_mode(&mut self) {
        self.move_line_start();
        self.enter_edit_mode();
    }

    pub fn enter_line_end_insert_mode(&mut self) {
        self.enter_edit_mode();
        self.move_line_end();
    }

    pub fn delete_to_line_end(&mut self) {
        let line_idx = self.current_field();
        let cursor = self.cursor_position();
        let current = self.current_text().to_string();
        let line_len = current.chars().count();

        if cursor >= line_len {
            return;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);

        let kept: String = current.chars().take(cursor).collect();
        self.editor
            .data_provider_mut()
            .set_field_value(line_idx, kept);
        self.set_cursor_position(cursor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    pub fn change_to_line_end(&mut self) {
        let cursor = self.cursor_position();
        self.delete_to_line_end();
        self.enter_edit_mode();
        self.set_cursor_position(cursor);
    }

    pub fn delete_current_line(&mut self) {
        self.delete_current_lines(1);
    }

    pub fn delete_current_lines(&mut self, count: usize) {
        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let current_line = self.current_field();
        let mut lines = self.editor.data_provider().capture_content();
        let count = count.max(1);

        if lines.len() <= 1 {
            self.editor.data_provider_mut().set_text(String::new());
            self.set_cursor_position(0);
        } else {
            let remove_idx = current_line.min(lines.len() - 1);
            let end = remove_idx.saturating_add(count).min(lines.len());
            lines.drain(remove_idx..end);
            if lines.is_empty() {
                lines.push(String::new());
            }
            self.editor.data_provider_mut().restore_content(&lines);
            let target = remove_idx.min(lines.len() - 1);
            let _ = self.transition_to_field(target);
            self.move_line_start();
        }

        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    pub fn change_current_line(&mut self) {
        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let line_idx = self.current_field();
        self.editor
            .data_provider_mut()
            .set_field_value(line_idx, String::new());
        self.move_line_start();
        self.enter_edit_mode();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    pub fn join_line_below(&mut self) {
        self.join_lines_below(1);
    }

    pub fn join_lines_below(&mut self, count: usize) {
        let line_idx = self.current_field();
        if line_idx + 1 >= self.editor.data_provider().field_count() {
            return;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut last_col = None;
        for _ in 0..count.max(1) {
            if let Some(new_col) = self.editor.data_provider_mut().join_with_next(line_idx) {
                last_col = Some(new_col);
            } else {
                break;
            }
        }

        if let Some(new_col) = last_col {
            self.set_cursor_position(new_col);
        }
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn half_page_lines(&self) -> usize {
        #[cfg(feature = "gui")]
        {
            return (self.viewport_height / 2).max(1) as usize;
        }
        #[cfg(not(feature = "gui"))]
        {
            5
        }
    }

    pub fn move_half_page_up(&mut self, count: usize) {
        let steps = self.half_page_lines().saturating_mul(count.max(1));
        for _ in 0..steps {
            if !self.move_up() {
                break;
            }
        }
    }

    pub fn move_half_page_down(&mut self, count: usize) {
        let steps = self.half_page_lines().saturating_mul(count.max(1));
        for _ in 0..steps {
            if !self.move_down() {
                break;
            }
        }
    }
}
