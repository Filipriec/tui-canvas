use crate::widgets::textarea::{TextAreaDataProvider, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn insert_newline(&mut self) {
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        let line_idx = self.current_field();
        let col = self.cursor_position();

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let new_idx = self.editor.data_provider_mut().split_line_at(line_idx, col);

        let _ = self.transition_to_field(new_idx);
        self.move_line_start();
        self.enter_edit_mode();
    }

    pub fn backspace(&mut self) {
        let col = self.cursor_position();
        if col > 0 {
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            let _ = self.delete_backward();
            return;
        }

        let line_idx = self.current_field();
        if line_idx == 0 {
            return;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        if let Some((prev_idx, new_col)) = self.editor.data_provider_mut().join_with_prev(line_idx)
        {
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            let _ = self.transition_to_field(prev_idx);
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
    }

    pub fn delete_forward_or_join(&mut self) {
        let line_idx = self.current_field();
        let line_len = self.current_text().chars().count();
        let col = self.cursor_position();

        if col < line_len {
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            let _ = self.delete_forward();
            return;
        }

        if line_idx + 1 < self.editor.data_provider().field_count() {
            self.editor
                .record_checkpoint(crate::editor::features::history::EditKind::Other);
        }

        if let Some(new_col) = self.editor.data_provider_mut().join_with_next(line_idx) {
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
    }

    #[cfg(feature = "keybindings")]
    pub(crate) fn delete_backward_preserving_mode(&mut self) {
        let previous_mode = self.mode();
        self.enter_edit_mode();
        self.backspace();
        self.set_mode(previous_mode);
    }

    #[cfg(feature = "keybindings")]
    pub(crate) fn delete_forward_preserving_mode(&mut self) {
        let previous_mode = self.mode();
        self.enter_edit_mode();
        self.delete_forward_or_join();
        self.set_mode(previous_mode);
    }

    pub(crate) fn insert_tab_spaces(&mut self) {
        self.enter_edit_mode();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        for _ in 0..4 {
            let _ = self.insert_char(' ');
        }
    }

    pub fn open_line_below(&mut self) {
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let line_idx = self.current_field();
        let new_idx = self
            .editor
            .data_provider_mut()
            .insert_blank_line_after(line_idx);

        let _ = self.transition_to_field(new_idx);
        self.move_line_start();
        self.enter_edit_mode();
    }

    pub fn open_line_above(&mut self) {
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }

        self.editor
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let line_idx = self.current_field();
        let new_idx = self
            .editor
            .data_provider_mut()
            .insert_blank_line_before(line_idx);

        let _ = self.transition_to_field(new_idx);
        self.move_line_start();
        self.enter_edit_mode();
    }
}
