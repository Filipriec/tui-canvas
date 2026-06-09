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

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);

        let kept: String = current.chars().take(cursor).collect();
        self.core
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
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let current_line = self.current_field();
        let mut lines = self.core.data_provider().capture_content();
        let count = count.max(1);

        if lines.len() <= 1 {
            self.core.data_provider_mut().set_text(String::new());
            self.set_cursor_position(0);
        } else {
            let remove_idx = current_line.min(lines.len() - 1);
            let end = remove_idx.saturating_add(count).min(lines.len());
            lines.drain(remove_idx..end);
            if lines.is_empty() {
                lines.push(String::new());
            }
            self.core.data_provider_mut().restore_content(&lines);
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
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let line_idx = self.current_field();
        self.core
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
        if line_idx + 1 >= self.core.data_provider().field_count() {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut last_col = None;
        for _ in 0..count.max(1) {
            if let Some(new_col) = self.core.data_provider_mut().join_with_next(line_idx) {
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

    /// Helix `J`: join the next line with a single space, dropping the next
    /// line's leading whitespace, leaving the cursor on the inserted space.
    pub fn join_lines_below_helix(&mut self, count: usize) {
        let line_idx = self.current_field();
        if line_idx + 1 >= self.core.data_provider().field_count() {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut content = self.core.data_provider().capture_content();
        let mut cursor_col = self.cursor_position();
        for _ in 0..count.max(1) {
            if line_idx + 1 >= content.len() {
                break;
            }
            let cur = content[line_idx].trim_end().to_string();
            let next = content[line_idx + 1].trim_start().to_string();
            cursor_col = cur.chars().count();
            content[line_idx] = if cur.is_empty() || next.is_empty() {
                format!("{cur}{next}")
            } else {
                format!("{cur} {next}")
            };
            content.remove(line_idx + 1);
        }
        self.core.data_provider_mut().restore_content(&content);

        let _ = self.transition_to_field(line_idx);
        self.set_cursor_position(cursor_col);
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// VSCode `Alt+Up`: move the current line up by `count` positions, carrying
    /// the cursor with it. No-op at the top of the buffer.
    pub fn move_line_up(&mut self, count: usize) {
        let cur = self.current_field();
        if cur == 0 {
            return;
        }
        let target = cur.saturating_sub(count.max(1));
        self.relocate_current_line(target);
    }

    /// VSCode `Alt+Down`: move the current line down by `count` positions,
    /// carrying the cursor with it. No-op at the bottom of the buffer.
    pub fn move_line_down(&mut self, count: usize) {
        let cur = self.current_field();
        let last = self.core.data_provider().field_count().saturating_sub(1);
        if cur >= last {
            return;
        }
        let target = (cur + count.max(1)).min(last);
        self.relocate_current_line(target);
    }

    /// Remove the current line and re-insert it at `target`, preserving the
    /// cursor column. Shared by `move_line_up`/`move_line_down`.
    fn relocate_current_line(&mut self, target: usize) {
        let cur = self.current_field();
        if cur == target {
            return;
        }
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let col = self.cursor_position();
        let mut lines = self.core.data_provider().capture_content();
        if cur >= lines.len() {
            return;
        }
        let line = lines.remove(cur);
        let target = target.min(lines.len());
        lines.insert(target, line);
        self.core.data_provider_mut().restore_content(&lines);

        let _ = self.transition_to_field(target);
        self.set_cursor_position(col);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// VSCode `Shift+Alt+Down`: duplicate the current line below, moving the
    /// cursor onto the new copy.
    pub fn duplicate_line_down(&mut self, count: usize) {
        self.duplicate_current_line(count, true);
    }

    /// VSCode `Shift+Alt+Up`: duplicate the current line above, leaving the
    /// cursor on the upper copy.
    pub fn duplicate_line_up(&mut self, count: usize) {
        self.duplicate_current_line(count, false);
    }

    fn duplicate_current_line(&mut self, count: usize, downward: bool) {
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let col = self.cursor_position();
        let cur = self.current_field();
        let mut lines = self.core.data_provider().capture_content();
        if cur >= lines.len() {
            return;
        }
        let line = lines[cur].clone();
        let copies = count.max(1);
        let insert_at = if downward { cur + 1 } else { cur };
        for _ in 0..copies {
            lines.insert(insert_at.min(lines.len()), line.clone());
        }
        self.core.data_provider_mut().restore_content(&lines);

        // Downward leaves the cursor on the first new copy; upward keeps it on
        // the topmost copy, which now sits at the original index.
        let target = if downward { cur + 1 } else { cur };
        let _ = self.transition_to_field(target);
        self.set_cursor_position(col);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// VSCode `Ctrl+C` with no selection: copy the whole current line (with a
    /// trailing newline) into the yank register and the OS clipboard. The
    /// selection-aware variant is handled in the VSCode dispatch.
    #[cfg(feature = "keybindings")]
    pub(crate) fn copy_current_line(&mut self) {
        let line = self.current_text().to_string();
        self.core
            .behavior_state
            .yank_mut()
            .set_line_register(vec![line]);
    }

    /// VSCode `Ctrl+X` with no selection: copy then delete the whole current
    /// line. The selection-aware variant is handled in the VSCode dispatch.
    #[cfg(feature = "keybindings")]
    pub(crate) fn cut_current_line(&mut self) {
        self.copy_current_line();
        self.delete_current_lines(1);
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

    fn page_lines(&self) -> usize {
        #[cfg(feature = "gui")]
        {
            return (self.viewport_height as usize).max(1);
        }
        #[cfg(not(feature = "gui"))]
        {
            10
        }
    }

    pub fn move_page_up(&mut self, count: usize) {
        let steps = self.page_lines().saturating_mul(count.max(1));
        for _ in 0..steps {
            if !self.move_up() {
                break;
            }
        }
    }

    pub fn move_page_down(&mut self, count: usize) {
        let steps = self.page_lines().saturating_mul(count.max(1));
        for _ in 0..steps {
            if !self.move_down() {
                break;
            }
        }
    }
}
