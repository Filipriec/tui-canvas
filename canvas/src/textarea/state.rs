// src/textarea/state.rs
use crate::editor::FormEditor;
use crate::textarea::provider::TextAreaProvider;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

pub type TextAreaEditor = FormEditor<TextAreaProvider>;

pub struct TextAreaState {
    pub(crate) editor: TextAreaEditor,
    pub(crate) scroll_y: u16,
    pub(crate) wrap: bool,
    pub(crate) placeholder: Option<String>,
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(TextAreaProvider::default()),
            scroll_y: 0,
            wrap: false,
            placeholder: None,
        }
    }
}

impl TextAreaState {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let provider = TextAreaProvider::from_text(text);
        Self {
            editor: FormEditor::new(provider),
            scroll_y: 0,
            wrap: false,
            placeholder: None,
        }
    }

    pub fn text(&self) -> String {
        self.editor.data_provider().to_text()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.editor.data_provider_mut().set_text(text);
        // Reset to first line and col 0
        self.editor.ui_state.current_field = 0;
        self.editor.ui_state.cursor_pos = 0;
        self.editor.ui_state.ideal_cursor_column = 0;
    }

    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
    }

    // Editing primitives specific to multi-line buffer
    pub fn insert_newline(&mut self) {
        let line_idx = self.editor.current_field();
        let col = self.editor.cursor_position();

        let new_idx = self
            .editor
            .data_provider_mut()
            .split_line_at(line_idx, col);

        let _ = self.editor.transition_to_field(new_idx);
        self.editor.move_line_start();
        self.editor.enter_edit_mode();
    }

    pub fn backspace(&mut self) {
        let col = self.editor.cursor_position();
        if col > 0 {
            let _ = self.editor.delete_backward();
            return;
        }

        let line_idx = self.editor.current_field();
        if line_idx == 0 {
            return;
        }

        if let Some((prev_idx, new_col)) = self
            .editor
            .data_provider_mut()
            .join_with_prev(line_idx)
        {
            let _ = self.editor.transition_to_field(prev_idx);
            self.editor.set_cursor_position(new_col);
            self.editor.enter_edit_mode();
        }
    }

    pub fn delete_forward_or_join(&mut self) {
        let line_idx = self.editor.current_field();
        let line_len = self.editor.current_text().chars().count();
        let col = self.editor.cursor_position();

        if col < line_len {
            let _ = self.editor.delete_forward();
            return;
        }

        if let Some(new_col) = self
            .editor
            .data_provider_mut()
            .join_with_next(line_idx)
        {
            self.editor.set_cursor_position(new_col);
            self.editor.enter_edit_mode();
        }
    }

    // Drive the editor from key events
    pub fn input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => self.insert_newline(),
            (KeyCode::Backspace, _) => self.backspace(),
            (KeyCode::Delete, _) => self.delete_forward_or_join(),

            (KeyCode::Left, _) => {
                let _ = self.editor.move_left();
            }
            (KeyCode::Right, _) => {
                let _ = self.editor.move_right();
            }
            (KeyCode::Up, _) => {
                let _ = self.editor.move_up();
            }
            (KeyCode::Down, _) => {
                let _ = self.editor.move_down();
            }

            (KeyCode::Home, _)
            | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.editor.move_line_start();
            }
            (KeyCode::End, _)
            | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.editor.move_line_end();
            }

            // Optional: word motions
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                self.editor.move_word_prev();
            }
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                self.editor.move_word_next();
            }
            (KeyCode::Char('e'), KeyModifiers::ALT) => {
                self.editor.move_word_end();
            }

            // Insert printable characters
            (KeyCode::Char(c), m) if m.is_empty() => {
                self.editor.enter_edit_mode();
                let _ = self.editor.insert_char(c);
            }

            // Tab: insert 4 spaces (simple default)
            (KeyCode::Tab, _) => {
                self.editor.enter_edit_mode();
                for _ in 0..4 {
                    let _ = self.editor.insert_char(' ');
                }
            }

            _ => {}
        }
    }

    // Cursor helpers for GUI
    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        let inner = if let Some(b) = block { b.inner(area) } else { area };
        let line_idx = self.editor.current_field() as u16;
        let y = inner.y + line_idx.saturating_sub(self.scroll_y);

        let current_line = self.editor.current_text();
        let col = self.editor.display_cursor_position();

        let mut x_off: u16 = 0;
        for (i, ch) in current_line.chars().enumerate() {
            if i >= col {
                break;
            }
            x_off = x_off.saturating_add(
                UnicodeWidthChar::width(ch).unwrap_or(0) as u16,
            );
        }
        let x = inner.x.saturating_add(x_off);
        (x, y)
    }

    #[cfg(feature = "gui")]
    pub(crate) fn ensure_visible(
        &mut self,
        area: Rect,
        block: Option<&Block<'_>>,
    ) {
        let inner = if let Some(b) = block { b.inner(area) } else { area };
        if inner.height == 0 {
            return;
        }
        let line_idx = self.editor.current_field() as u16;
        if line_idx < self.scroll_y {
            self.scroll_y = line_idx;
        } else if line_idx >= self.scroll_y + inner.height {
            self.scroll_y = line_idx.saturating_sub(inner.height - 1);
        }
    }
}
