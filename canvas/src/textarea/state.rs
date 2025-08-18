// src/textarea/state.rs
use std::ops::{Deref, DerefMut};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::editor::FormEditor;
use crate::textarea::provider::TextAreaProvider;
use crate::data_provider::DataProvider;

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
fn wrapped_rows(s: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let mut rows: u16 = 1;
    let mut cols: u16 = 0;
    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if cols.saturating_add(w) > width {
            rows = rows.saturating_add(1);
            cols = 0;
        }
        cols = cols.saturating_add(w);
    }
    rows
}

#[cfg(feature = "gui")]
fn wrapped_rows_to_cursor(s: &str, width: u16, cursor_chars: usize) -> (u16, u16) {
    if width == 0 {
        return (0, 0);
    }
    let mut row: u16 = 0;
    let mut cols: u16 = 0;
    for (i, ch) in s.chars().enumerate() {
        if i >= cursor_chars {
            break;
        }
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if cols.saturating_add(w) > width {
            row = row.saturating_add(1);
            cols = 0;
        }
        cols = cols.saturating_add(w);
    }
    (row, cols)
}

pub type TextAreaEditor = FormEditor<TextAreaProvider>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflowMode {
    Indicator { ch: char }, // show trailing indicator (default '$')
    Wrap,                    // soft wrap lines
}

pub struct TextAreaState {
    pub(crate) editor: TextAreaEditor,
    pub(crate) scroll_y: u16,
    pub(crate) placeholder: Option<String>,
    pub(crate) overflow_mode: TextOverflowMode,
    pub(crate) h_scroll: u16,
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(TextAreaProvider::default()),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
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
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
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

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
    }

    // RUNTIME TOGGLES ----------------------------------------------------

    pub fn use_overflow_indicator(&mut self, ch: char) {
        self.overflow_mode = TextOverflowMode::Indicator { ch };
    }

    pub fn use_wrap(&mut self) {
        self.overflow_mode = TextOverflowMode::Wrap;
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

        if let Some((prev_idx, new_col)) =
            self.editor.data_provider_mut().join_with_prev(line_idx)
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

        if let Some(new_col) =
            self.editor.data_provider_mut().join_with_next(line_idx)
        {
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
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

            // Optional: word motions (kept)
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
        let line_idx = self.current_field() as usize;

        match self.overflow_mode {
            TextOverflowMode::Wrap => {
                let width = inner.width;
                // Visual rows above the current line (from the first visible line)
                let mut rows_above: u16 = 0;
                for i in (self.scroll_y as usize)..line_idx {
                    rows_above = rows_above
                        .saturating_add(wrapped_rows(
                            self.editor.data_provider().field_value(i),
                            width,
                        ));
                }

                let current_line = self.current_text();
                let col_chars = self.display_cursor_position();
                let (row_in_line, col_in_row) =
                    wrapped_rows_to_cursor(&current_line, width, col_chars);

                let y = inner.y.saturating_add(rows_above).saturating_add(row_in_line);
                let x = inner.x.saturating_add(col_in_row);
                (x, y)
            }
            TextOverflowMode::Indicator { .. } => {
                // existing indicator path (with h_scroll)
                let y = inner.y
                    + (line_idx as u16)
                        .saturating_sub(self.scroll_y);
                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let mut x_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    x_cols = x_cols
                        .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }
                let x_off_visible = x_cols.saturating_sub(self.h_scroll);
                let x = inner.x.saturating_add(x_off_visible);
                (x, y)
            }
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn ensure_visible(&mut self, area: Rect, block: Option<&Block<'_>>) {
        let inner = if let Some(b) = block { b.inner(area) } else { area };
        if inner.height == 0 {
            return;
        }

        // Keep logical line within vertical window (coarse guard)
        let line_idx_u16 = self.current_field() as u16;
        if line_idx_u16 < self.scroll_y {
            self.scroll_y = line_idx_u16;
        } else if line_idx_u16 >= self.scroll_y + inner.height {
            self.scroll_y = line_idx_u16.saturating_sub(inner.height - 1);
        }

        match self.overflow_mode {
            TextOverflowMode::Indicator { .. } => {
                let width = inner.width;
                if width == 0 {
                    return;
                }

                // If the line fits, drop any horizontal scroll
                let current_line = self.current_text();
                let mut total_cols: u16 = 0;
                for ch in current_line.chars() {
                    total_cols = total_cols
                        .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }
                if total_cols <= width {
                    self.h_scroll = 0;
                    return;
                }

                // Follow caret with right padding reserved
                let col = self.display_cursor_position();
                let mut cursor_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    cursor_cols = cursor_cols
                        .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }

                let right_padding: u16 = 3;
                // reserve 1 column for a potential right indicator
                let visible_limit = width.saturating_sub(1 + right_padding);

                if cursor_cols > self.h_scroll.saturating_add(visible_limit) {
                    self.h_scroll = cursor_cols.saturating_sub(visible_limit);
                } else if cursor_cols < self.h_scroll {
                    self.h_scroll = cursor_cols;
                }
            }
            TextOverflowMode::Wrap => {
                self.h_scroll = 0; // no horizontal scroll in wrap

                let width = inner.width;
                if width == 0 {
                    return;
                }

                // Ensure the cursor's wrapped row is on screen
                let current_idx = self.current_field();
                // Visual rows above current line from current scroll_y
                let mut rows_above: u16 = 0;
                for i in (self.scroll_y as usize)..current_idx {
                    rows_above = rows_above
                        .saturating_add(wrapped_rows(
                            self.editor.data_provider().field_value(i),
                            width,
                        ));
                }

                // Cursor's row within current line
                let (row_in_line, _) = wrapped_rows_to_cursor(
                    &self.current_text(),
                    width,
                    self.display_cursor_position(),
                );

                // Scroll down if cursor row is below the visible window
                while rows_above.saturating_add(row_in_line) >= inner.height {
                    if self.scroll_y < current_idx as u16 {
                        // subtract the rows of the line we're dropping from the top
                        let dropped = wrapped_rows(
                            self.editor
                                .data_provider()
                                .field_value(self.scroll_y as usize),
                            width,
                        );
                        self.scroll_y = self.scroll_y.saturating_add(1);
                        rows_above = rows_above.saturating_sub(dropped);
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
