#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    editor::behavior::YankRegister,
    editor::paste::repeated_text,
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
        self.core.paste_register_lines_core(after, count, lines);
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
        self.core.paste_register_lines_core(after, count, lines);
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
        if self.mode() == AppMode::Sel {
            self.exit_highlight_mode_emacs();
        }
        self.core.paste_register_lines_core(after, count, lines);
        self.set_mode(AppMode::Nor);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    fn insert_text_at(&mut self, field: usize, col: usize, text: &str) -> (usize, usize) {
        self.core.insert_register_text_core(field, col, text)
    }
}
