// src/textarea/state.rs
use std::ops::{Deref, DerefMut};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::editor::FormEditor;
use crate::textarea::provider::TextAreaProvider;

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

// Expose the entire FormEditor API directly on TextAreaState
impl Deref for TextAreaState {
    type Target = TextAreaEditor;

    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl DerefMut for TextAreaState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
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

    // Textarea-specific primitive: split at cursor
    pub fn insert_newline(&mut self) {
        let line_idx = self.current_field();
        let col = self.cursor_position();

        let new_idx = self
            .editor
            .data_provider_mut()
            .split_line_at(line_idx, col);

        let _ = self.transition_to_field(new_idx);
        self.move_line_start();
        self.enter_edit_mode();
    }

    // Textarea-specific primitive: backspace with line join at start-of-line
    pub fn backspace(&mut self) {
        let col = self.cursor_position();
        if col > 0 {
            let _ = self.delete_backward();
            return;
        }

        let line_idx = self.current_field();
        if line_idx == 0 {
            return;
        }

        if let Some((prev_idx, new_col)) = self
            .editor
            .data_provider_mut()
            .join_with_prev(line_idx)
        {
            let _ = self.transition_to_field(prev_idx);
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
    }

    // Textarea-specific primitive: delete or join with next line at EOL
    pub fn delete_forward_or_join(&mut self) {
        let line_idx = self.current_field();
        let line_len = self.current_text().chars().count();
        let col = self.cursor_position();

        if col < line_len {
            let _ = self.delete_forward();
            return;
        }

        if let Some(new_col) = self
            .editor
            .data_provider_mut()
            .join_with_next(line_idx)
        {
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
    }

    // Override for multiline: insert new blank line below and enter insert mode.
    pub fn open_line_below(&mut self) -> Result<()> {
        let line_idx = self.current_field();
        let new_idx = self
            .editor
            .data_provider_mut()
            .insert_blank_line_after(line_idx);

        self.transition_to_field(new_idx)?;
        self.move_line_start();
        self.enter_edit_mode();
        Ok(())
    }

    // Override for multiline: insert new blank line above and enter insert mode.
    pub fn open_line_above(&mut self) -> Result<()> {
        let line_idx = self.current_field();
        let new_idx = self
            .editor
            .data_provider_mut()
            .insert_blank_line_before(line_idx);

        self.transition_to_field(new_idx)?;
        self.move_line_start();
        self.enter_edit_mode();
        Ok(())
    }

    // Drive from KeyEvent; you can still call all FormEditor methods directly
    pub fn input(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => self.insert_newline(),
            (KeyCode::Backspace, _) => self.backspace(),
            (KeyCode::Delete, _) => self.delete_forward_or_join(),

            (KeyCode::Left, _) => {
                let _ = self.move_left();
            }
            (KeyCode::Right, _) => {
                let _ = self.move_right();
            }
            (KeyCode::Up, _) => {
                let _ = self.move_up();
            }
            (KeyCode::Down, _) => {
                let _ = self.move_down();
            }

            (KeyCode::Home, _)
            | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_line_start();
            }
            (KeyCode::End, _)
            | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_line_end();
            }

            // Optional: word motions
            (KeyCode::Char('b'), KeyModifiers::ALT) => self.move_word_prev(),
            (KeyCode::Char('f'), KeyModifiers::ALT) => self.move_word_next(),
            (KeyCode::Char('e'), KeyModifiers::ALT) => self.move_word_end(),

            // Printable characters
            (KeyCode::Char(c), m) if m.is_empty() => {
                self.enter_edit_mode();
                let _ = self.insert_char(c);
            }

            // Simple Tab policy
            (KeyCode::Tab, _) => {
                self.enter_edit_mode();
                for _ in 0..4 {
                    let _ = self.insert_char(' ');
                }
            }

            _ => {}
        }
    }

    // Cursor helpers for GUI
    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        let inner = if let Some(b) = block { b.inner(area) } else { area };
        let line_idx = self.current_field() as u16;
        let y = inner.y + line_idx.saturating_sub(self.scroll_y);

        let current_line = self.current_text();
        let col = self.display_cursor_position();

        let mut x_off: u16 = 0;
        for (i, ch) in current_line.chars().enumerate() {
            if i >= col {
                break;
            }
            x_off = x_off
                .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
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
        let line_idx = self.current_field() as u16;
        if line_idx < self.scroll_y {
            self.scroll_y = line_idx;
        } else if line_idx >= self.scroll_y + inner.height {
            self.scroll_y = line_idx.saturating_sub(inner.height - 1);
        }
    }
}
