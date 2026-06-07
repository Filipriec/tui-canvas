#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    editor::behavior::YankRegister,
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

    pub(crate) fn paste_after_emacs(&mut self, count: usize) {
        self.paste_yank_emacs(true, count);
    }

    pub(crate) fn paste_before_emacs(&mut self, count: usize) {
        self.paste_yank_emacs(false, count);
    }

    fn paste_yank_vim(&mut self, after: bool, count: usize) {
        let Some(register) = self.core.behavior_state.yank().register().cloned() else {
            return;
        };

        match register {
            YankRegister::Lines(lines) => self.paste_lines_vim(after, count, lines),
            YankRegister::Text(lines) => {
                let text = repeated_text(&lines, count);
                if text.is_empty() {
                    return;
                }
                let field = self.current_field();
                let line_len = self.current_text().chars().count();
                let col = if after {
                    self.cursor_position().saturating_add(1).min(line_len)
                } else {
                    self.cursor_position()
                };
                let (target_field, target_col) = self.insert_text_at(field, col, &text);
                let _ = self.transition_to_field(target_field);
                self.set_cursor_position(target_col);
                self.set_mode(AppMode::Nor);
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
            }
        }
    }

    fn paste_yank_helix(&mut self, after: bool, count: usize) {
        let Some(register) = self.core.behavior_state.yank().register().cloned() else {
            return;
        };

        match register {
            YankRegister::Lines(lines) => self.paste_lines_helix(after, count, lines),
            YankRegister::Text(lines) => {
                let text = repeated_text(&lines, count);
                if text.is_empty() {
                    return;
                }
                let (start, end) = self.selection_endpoints();
                let (field, col) = if after {
                    (end.0, end.1.saturating_add(1))
                } else {
                    start
                };
                let (target_field, target_col) = self.insert_text_at(field, col, &text);
                let _ = self.transition_to_field(target_field);
                self.set_cursor_position(target_col);
                self.core.ui_state.current_mode = AppMode::Nor;
                self.ensure_helix_primary_selection();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
            }
        }
    }

    fn paste_yank_emacs(&mut self, after: bool, count: usize) {
        let Some(register) = self.core.behavior_state.yank().register().cloned() else {
            return;
        };

        match register {
            YankRegister::Lines(lines) => self.paste_lines_emacs(after, count, lines),
            YankRegister::Text(lines) => {
                let text = repeated_text(&lines, count);
                if text.is_empty() {
                    return;
                }
                if self.mode() == AppMode::Sel {
                    self.exit_highlight_mode_emacs();
                }
                let (target_field, target_col) =
                    self.insert_text_at(self.current_field(), self.cursor_position(), &text);
                let _ = self.transition_to_field(target_field);
                self.set_cursor_position(target_col);
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
            }
        }
    }

    fn paste_lines_vim(&mut self, after: bool, count: usize, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut content = self.core.data_provider().capture_content();
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
        self.core.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
        self.move_line_start();
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn paste_lines_helix(&mut self, after: bool, count: usize, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut content = self.core.data_provider().capture_content();
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
        self.core.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
        self.move_line_start();
        self.core.ui_state.current_mode = AppMode::Nor;
        self.ensure_helix_primary_selection();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn paste_lines_emacs(&mut self, after: bool, count: usize, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        if self.mode() == AppMode::Sel {
            self.exit_highlight_mode_emacs();
        }

        let mut content = self.core.data_provider().capture_content();
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
        self.core.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
        self.move_line_start();
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn insert_text_at(&mut self, field: usize, col: usize, text: &str) -> (usize, usize) {
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut content = self.core.data_provider().capture_content();
        if content.is_empty() {
            content.push(String::new());
        }
        let field = field.min(content.len().saturating_sub(1));
        let line = &content[field];
        let col = col.min(line.chars().count());
        let prefix: String = line.chars().take(col).collect();
        let suffix: String = line.chars().skip(col).collect();
        let parts: Vec<&str> = text.split('\n').collect();

        let target = if parts.len() == 1 {
            content[field] = format!("{prefix}{}{suffix}", parts[0]);
            (field, col.saturating_add(parts[0].chars().count()))
        } else {
            let mut replacement = Vec::with_capacity(parts.len());
            replacement.push(format!("{prefix}{}", parts[0]));
            for part in &parts[1..parts.len() - 1] {
                replacement.push((*part).to_string());
            }
            let last = parts[parts.len() - 1];
            replacement.push(format!("{last}{suffix}"));
            content.splice(field..=field, replacement);
            (field.saturating_add(parts.len() - 1), last.chars().count())
        };

        self.core.data_provider_mut().restore_content(&content);
        target
    }
}

fn repeated_text(lines: &[String], count: usize) -> String {
    let repeat = count.max(1);
    let text = lines.join("\n");
    let mut pasted = String::new();
    for i in 0..repeat {
        if i > 0 {
            pasted.push('\n');
        }
        pasted.push_str(&text);
    }
    pasted
}
