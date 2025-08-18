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
pub(crate) fn wrapped_rows(s: &str, width: u16) -> u16 {
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
pub(crate) fn wrapped_rows_to_cursor(s: &str, width: u16, cursor_chars: usize) -> (u16, u16) {
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

#[cfg(feature = "gui")]
pub(crate) const RIGHT_PAD: u16 = 3;

#[cfg(feature = "gui")]
pub(crate) fn compute_h_scroll_with_padding(
    cursor_cols: u16,
    width: u16,
) -> (u16, u16) {
    // Returns (h_scroll, left_cols). left_cols = 1 if a left indicator is shown.
    let mut h = 0u16;
    for _ in 0..2 {
        let left_cols = if h > 0 { 1 } else { 0 };
        let max_x_visible = width.saturating_sub(1 + RIGHT_PAD + left_cols);
        let needed = cursor_cols.saturating_sub(max_x_visible);
        if needed <= h {
            return (h, left_cols);
        }
        h = needed;
    }
    let left_cols = if h > 0 { 1 } else { 0 };
    (h, left_cols)
}

#[cfg(feature = "gui")]
fn normalize_indent(width: u16, indent: u16) -> u16 {
    // Ensure continuation capacity stays >= 1
    indent.min(width.saturating_sub(1))
}

// Count visual rows for a single logical line using early-wrap and continuation indent
#[cfg(feature = "gui")]
pub(crate) fn count_wrapped_rows_indented(
    s: &str,
    width: u16,
    indent: u16,
) -> u16 {
    if width == 0 {
        return 1;
    }
    let indent = normalize_indent(width, indent);
    let cont_cap = width.saturating_sub(indent);

    let mut rows: u16 = 1;
    let mut used: u16 = 0;
    let mut first = true;

    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let cap = if first { width } else { cont_cap };

        // Early-wrap: avoid the "one char freeze" at the boundary
        if used > 0 && used.saturating_add(w) >= cap {
            rows = rows.saturating_add(1);
            first = false;
            used = indent; // continuation indent occupies leading cells
        }
        used = used.saturating_add(w);
    }

    rows
}

// Compute caret (subrow, x) for a given cursor index with indent + early-wrap
#[cfg(feature = "gui")]
fn wrapped_rows_to_cursor_indented(
    s: &str,
    width: u16,
    indent: u16,
    cursor_chars: usize,
) -> (u16, u16) {
    if width == 0 {
        return (0, 0);
    }
    let indent = normalize_indent(width, indent);
    let cont_cap = width.saturating_sub(indent);

    let mut row: u16 = 0;
    let mut used: u16 = 0;
    let mut first = true;

    for (i, ch) in s.chars().enumerate() {
        if i >= cursor_chars {
            break;
        }
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let cap = if first { width } else { cont_cap };

        if used > 0 && used.saturating_add(w) >= cap {
            row = row.saturating_add(1);
            first = false;
            used = indent; // place indent on continuation line
        }
        used = used.saturating_add(w);
    }

    // 'used' already includes indent when on continuation rows
    (row, used.min(width.saturating_sub(1)))
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
    // NEW: visual indentation for wrapped continuation rows (Vim-like)
    #[cfg(feature = "gui")]
    pub(crate) wrap_indent_cols: u16,
}

impl Default for TextAreaState {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(TextAreaProvider::default()),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0, // default: no continuation indent
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
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0,
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

    // Optional: set continuation indent for wrap mode (e.g. 3 like Vim)
    pub fn set_wrap_indent_cols(&mut self, cols: u16) {
        #[cfg(feature = "gui")]
        {
            self.wrap_indent_cols = cols;
        }
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

    // -----------------------------------------------------------------
    // Cursor helpers for GUI
    // -----------------------------------------------------------------

    #[cfg(feature = "gui")]
    fn visual_rows_before_line_indented(
        &self,
        width: u16,
        line_idx: usize,
    ) -> u16 {
        let provider = self.editor.data_provider();
        let mut acc: u16 = 0;
        let indent = self.wrap_indent_cols;

        for i in 0..line_idx {
            let s = provider.field_value(i);
            acc = acc.saturating_add(count_wrapped_rows_indented(s, width, indent));
        }
        acc
    }

    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        let inner = if let Some(b) = block { b.inner(area) } else { area };
        let line_idx = self.current_field() as usize;

        match self.overflow_mode {
            TextOverflowMode::Wrap => {
                let width = inner.width;
                let y_top = inner.y;
                let indent = self.wrap_indent_cols;

                if width == 0 {
                    let prefix = self.visual_rows_before_line_indented(1, line_idx);
                    let y = y_top.saturating_add(prefix.saturating_sub(self.scroll_y));
                    return (inner.x, y);
                }

                let prefix_rows =
                    self.visual_rows_before_line_indented(width, line_idx);
                let current_line = self.current_text();
                let col_chars = self.display_cursor_position();

                let (subrow, x_cols) = wrapped_rows_to_cursor_indented(
                    &current_line,
                    width,
                    indent,
                    col_chars,
                );

                let caret_vis_row = prefix_rows.saturating_add(subrow);
                let y = y_top.saturating_add(caret_vis_row.saturating_sub(self.scroll_y));
                let x = inner.x.saturating_add(x_cols);
                (x, y)
            }
            TextOverflowMode::Indicator { .. } => {
                let y = inner.y + (line_idx as u16).saturating_sub(self.scroll_y);
                let current_line = self.current_text();
                let col = self.display_cursor_position();

                // Display columns up to caret
                let mut x_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    x_cols = x_cols
                        .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }

                let left_cols = if self.h_scroll > 0 { 1 } else { 0 };

                let mut x_off_visible = x_cols
                    .saturating_sub(self.h_scroll)
                    .saturating_add(left_cols);

                let limit = inner.width.saturating_sub(1 + RIGHT_PAD);
                if x_off_visible > limit {
                    x_off_visible = limit;
                }

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

        match self.overflow_mode {
            TextOverflowMode::Indicator { .. } => {
                // Logical-line vertical scroll
                let line_idx_u16 = self.current_field() as u16;
                if line_idx_u16 < self.scroll_y {
                    self.scroll_y = line_idx_u16;
                } else if line_idx_u16 >= self.scroll_y + inner.height {
                    self.scroll_y = line_idx_u16.saturating_sub(inner.height - 1);
                }

                let width = inner.width;
                if width == 0 {
                    return;
                }

                let current_line = self.current_text();
                let col = self.display_cursor_position();
                let mut cursor_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    cursor_cols = cursor_cols
                        .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }

                let (target_h, _left_cols) =
                    compute_h_scroll_with_padding(cursor_cols, width);

                if target_h > self.h_scroll {
                    self.h_scroll = target_h;
                } else if cursor_cols < self.h_scroll {
                    self.h_scroll = cursor_cols;
                }
            }
            TextOverflowMode::Wrap => {
                let width = inner.width;
                if width == 0 {
                    self.h_scroll = 0;
                    return;
                }

                let indent = self.wrap_indent_cols;
                let line_idx = self.current_field() as usize;

                let prefix_rows =
                    self.visual_rows_before_line_indented(width, line_idx);

                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let (subrow, _x_cols) =
                    wrapped_rows_to_cursor_indented(&current_line, width, indent, col);

                let caret_vis_row = prefix_rows.saturating_add(subrow);

                let top = self.scroll_y;
                let height = inner.height;

                if caret_vis_row < top {
                    self.scroll_y = caret_vis_row;
                } else {
                    let bottom = top.saturating_add(height.saturating_sub(1));
                    if caret_vis_row > bottom {
                        let shift = caret_vis_row.saturating_sub(bottom);
                        self.scroll_y = top.saturating_add(shift);
                    }
                }

                self.h_scroll = 0; // no horizontal scroll in wrap mode
            }
        }
    }
}
