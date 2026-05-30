// src/textarea/state.rs
use std::ops::{Deref, DerefMut};

#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;
use crate::editor::FormEditor;
#[cfg(feature = "gui")]
use crate::gui_utils::{compute_h_scroll_with_padding, RIGHT_PAD};
use crate::textarea::provider::{TextAreaDataProvider, TextAreaProvider};
#[cfg(feature = "cursor-style")]
use std::io;

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
fn normalize_indent(width: u16, indent: u16) -> u16 {
    indent.min(width.saturating_sub(1))
}

#[cfg(feature = "gui")]
pub(crate) fn count_wrapped_rows_indented(s: &str, width: u16, indent: u16) -> u16 {
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

        if used > 0 && used.saturating_add(w) >= cap {
            rows = rows.saturating_add(1);
            first = false;
            used = indent;
        }
        used = used.saturating_add(w);
    }

    rows
}

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
            used = indent;
        }
        used = used.saturating_add(w);
    }

    (row, used.min(width.saturating_sub(1)))
}

pub type TextAreaEditor<P = TextAreaProvider> = FormEditor<P>;

/// Outcome of feeding a single input event to a [`TextAreaState`].
///
/// Unlike the single-line input there is no `Submitted` variant: in a textarea
/// `Enter` inserts a newline rather than submitting. Hosts can use `Ignored` to
/// detect keys the textarea did not consume (e.g. to drive focus handoff).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaEventOutcome {
    /// The event was not recognized/consumed by the textarea.
    Ignored,
    /// The event was handled (text edited or cursor moved).
    Handled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflowMode {
    Indicator { ch: char },
    Wrap,
}

/// Multi-line textarea widget state.
///
/// Wraps a [`FormEditor`]. Editing, cursor, and movement methods from the engine
/// are available directly, and the engine can be reached explicitly via
/// [`TextAreaState::editor`] / [`TextAreaState::editor_mut`]. With the
/// `validation` and `computed` features enabled, the corresponding helper
/// methods are re-exposed as inherent methods on this type.
pub struct TextAreaState<P: TextAreaDataProvider = TextAreaProvider> {
    pub(crate) editor: TextAreaEditor<P>,
    pub(crate) scroll_y: u16,
    pub(crate) placeholder: Option<String>,
    pub(crate) overflow_mode: TextOverflowMode,
    pub(crate) h_scroll: u16,
    #[cfg(feature = "gui")]
    pub(crate) wrap_indent_cols: u16,
    #[cfg(feature = "gui")]
    pub(crate) edited_this_frame: bool,
}

impl<P: TextAreaDataProvider + Default> Default for TextAreaState<P> {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(P::default()),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }
}

impl<P: TextAreaDataProvider> Deref for TextAreaState<P> {
    type Target = TextAreaEditor<P>;

    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl<P: TextAreaDataProvider> DerefMut for TextAreaState<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn with_provider(provider: P) -> Self {
        Self {
            editor: FormEditor::new(provider),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }

    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self::with_provider(P::from_text(text.into()))
    }

    pub fn text(&self) -> String {
        self.editor.data_provider().to_text()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.editor.data_provider_mut().set_text(text.into());
        self.editor.ui_state.current_field = 0;
        self.editor.set_cursor_raw(0);
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
    }

    pub fn use_overflow_indicator(&mut self, ch: char) {
        self.overflow_mode = TextOverflowMode::Indicator { ch };
    }

    pub fn use_wrap(&mut self) {
        self.overflow_mode = TextOverflowMode::Wrap;
    }

    pub fn set_wrap_indent_cols(&mut self, cols: u16) {
        #[cfg(feature = "gui")]
        {
            self.wrap_indent_cols = cols;
        }
    }

    pub fn insert_newline(&mut self) {
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        let line_idx = self.current_field();
        let col = self.cursor_position();

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

        if let Some(new_col) = self.editor.data_provider_mut().join_with_next(line_idx) {
            #[cfg(feature = "gui")]
            {
                self.edited_this_frame = true;
            }
            self.set_cursor_position(new_col);
            self.enter_edit_mode();
        }
    }

    pub fn paste(&mut self, text: &str) -> TextAreaEventOutcome {
        self.enter_edit_mode();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }

        for ch in text.chars() {
            match ch {
                '\r' => {}
                '\n' => self.insert_newline(),
                other => {
                    let _ = self.insert_char(other);
                }
            }
        }

        TextAreaEventOutcome::Handled
    }

    // TODO: Replace direct crossterm event coupling with a backend-agnostic
    // input abstraction so terminal input backends can be swapped cleanly.
    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> TextAreaEventOutcome {
        match event {
            Event::Key(key) => self.input(key),
            Event::Paste(text) => self.paste(&text),
            _ => TextAreaEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "crossterm")]
    pub fn input(&mut self, key: KeyEvent) -> TextAreaEventOutcome {
        if key.kind != KeyEventKind::Press {
            return TextAreaEventOutcome::Ignored;
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

            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_line_start();
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_line_end();
            }

            (KeyCode::Char('b'), KeyModifiers::ALT) => self.move_word_prev(),
            (KeyCode::Char('f'), KeyModifiers::ALT) => self.move_word_next(),
            (KeyCode::Char('e'), KeyModifiers::ALT) => self.move_word_end(),

            (KeyCode::Char(c), m) if m.is_empty() => {
                self.enter_edit_mode();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                let _ = self.insert_char(c);
            }

            (KeyCode::Tab, _) => {
                self.enter_edit_mode();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                for _ in 0..4 {
                    let _ = self.insert_char(' ');
                }
            }

            _ => return TextAreaEventOutcome::Ignored,
        }

        TextAreaEventOutcome::Handled
    }

    #[cfg(feature = "gui")]
    fn visual_rows_before_line_and_intra_indented(&self, width: u16, line_idx: usize) -> u16 {
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
        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        let line_idx = self.current_field();

        match self.overflow_mode {
            TextOverflowMode::Wrap => {
                let width = inner.width;
                let y_top = inner.y;
                let indent = self.wrap_indent_cols;

                if width == 0 {
                    let prefix = self.visual_rows_before_line_and_intra_indented(1, line_idx);
                    let y = y_top.saturating_add(prefix.saturating_sub(self.scroll_y));
                    return (inner.x, y);
                }

                let prefix_rows = self.visual_rows_before_line_and_intra_indented(width, line_idx);
                let current_line = self.current_text();
                let col_chars = self.display_cursor_position();

                let (subrow, x_cols) =
                    wrapped_rows_to_cursor_indented(current_line, width, indent, col_chars);

                let caret_vis_row = prefix_rows.saturating_add(subrow);
                let y = y_top.saturating_add(caret_vis_row.saturating_sub(self.scroll_y));
                let x = inner.x.saturating_add(x_cols);
                (x, y)
            }
            TextOverflowMode::Indicator { .. } => {
                let y = inner.y + (line_idx as u16).saturating_sub(self.scroll_y);
                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let mut x_cols: u16 = 0;
                let mut total_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if i < col {
                        x_cols = x_cols.saturating_add(w);
                    }
                    total_cols = total_cols.saturating_add(w);
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
        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        if inner.height == 0 {
            return;
        }

        match self.overflow_mode {
            TextOverflowMode::Indicator { .. } => {
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
                let mut total_cols: u16 = 0;
                for ch in current_line.chars() {
                    total_cols =
                        total_cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }
                if total_cols <= width {
                    self.h_scroll = 0;
                    return;
                }

                let col = self.display_cursor_position();
                let mut cursor_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    cursor_cols =
                        cursor_cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }

                let (target_h, _left_cols) = compute_h_scroll_with_padding(cursor_cols, width);

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
                let line_idx = self.current_field();

                let prefix_rows = self.visual_rows_before_line_and_intra_indented(width, line_idx);

                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let (subrow, _x_cols) =
                    wrapped_rows_to_cursor_indented(current_line, width, indent, col);

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

                self.h_scroll = 0;
            }
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn take_edited_flag(&mut self) -> bool {
        let v = self.edited_this_frame;
        self.edited_this_frame = false;
        v
    }
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Borrow the underlying [`FormEditor`] engine.
    ///
    /// `TextAreaState` is a thin wrapper around a `FormEditor`. Use this for
    /// engine functionality not re-exposed directly on the wrapper.
    pub fn editor(&self) -> &TextAreaEditor<P> {
        &self.editor
    }

    /// Mutably borrow the underlying [`FormEditor`] engine.
    pub fn editor_mut(&mut self) -> &mut TextAreaEditor<P> {
        &mut self.editor
    }

    /// Update the terminal cursor style to match the textarea's current mode.
    ///
    /// Unlike the single-line input (which is always insert-style), the textarea
    /// honours the editor's mode: in vim mode this yields a steady block cursor
    /// in normal/read-only mode and a bar cursor in edit mode. With the
    /// `cursor-style` feature disabled this is a no-op.
    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(self.editor.mode())
    }

    /// No-op when the `cursor-style` feature is disabled.
    #[cfg(not(feature = "cursor-style"))]
    pub fn update_cursor_style(&self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Validation helpers, re-exposed from the underlying [`FormEditor`] so they are
/// part of `TextAreaState`'s own public API rather than only reachable through
/// `Deref`.
#[cfg(feature = "validation")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn set_validation_enabled(&mut self, enabled: bool) {
        self.editor.set_validation_enabled(enabled);
    }

    pub fn is_validation_enabled(&self) -> bool {
        self.editor.is_validation_enabled()
    }

    pub fn set_field_validation(
        &mut self,
        field_index: usize,
        config: crate::validation::ValidationConfig,
    ) {
        self.editor.set_field_validation(field_index, config);
    }

    pub fn remove_field_validation(&mut self, field_index: usize) {
        self.editor.remove_field_validation(field_index);
    }

    pub fn validate_current_field(&mut self) -> crate::validation::ValidationResult {
        self.editor.validate_current_field()
    }

    pub fn validate_field(
        &mut self,
        field_index: usize,
    ) -> Option<crate::validation::ValidationResult> {
        self.editor.validate_field(field_index)
    }

    pub fn clear_validation_results(&mut self) {
        self.editor.clear_validation_results();
    }

    pub fn validation_summary(&self) -> crate::validation::ValidationSummary {
        self.editor.validation_summary()
    }

    pub fn can_switch_fields(&self) -> bool {
        self.editor.can_switch_fields()
    }

    pub fn field_switch_block_reason(&self) -> Option<String> {
        self.editor.field_switch_block_reason()
    }

    pub fn last_switch_block(&self) -> Option<&str> {
        self.editor.last_switch_block()
    }

    pub fn current_limits_status_text(&self) -> Option<String> {
        self.editor.current_limits_status_text()
    }

    pub fn current_formatter_warning(&self) -> Option<String> {
        self.editor.current_formatter_warning()
    }

    pub fn external_validation_of(
        &self,
        field_index: usize,
    ) -> crate::validation::ExternalValidationState {
        self.editor.external_validation_of(field_index)
    }

    pub fn clear_all_external_validation(&mut self) {
        self.editor.clear_all_external_validation();
    }

    pub fn clear_external_validation(&mut self, field_index: usize) {
        self.editor.clear_external_validation(field_index);
    }

    pub fn set_external_validation(
        &mut self,
        field_index: usize,
        state: crate::validation::ExternalValidationState,
    ) {
        self.editor.set_external_validation(field_index, state);
    }

    pub fn set_external_validation_callback<F>(&mut self, callback: F)
    where
        F: FnMut(usize, &str) -> crate::validation::ExternalValidationState + Send + Sync + 'static,
    {
        self.editor.set_external_validation_callback(callback);
    }
}

/// Computed-field helpers, re-exposed from the underlying [`FormEditor`].
#[cfg(feature = "computed")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn register_computed_provider<C>(&mut self, provider: &C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.register_computed_provider(provider);
    }

    pub fn set_computed_provider<C>(&mut self, provider: C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.set_computed_provider(provider);
    }

    pub fn recompute_fields<C>(&mut self, provider: &mut C, field_indices: &[usize])
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.recompute_fields(provider, field_indices);
    }

    pub fn recompute_all_fields<C>(&mut self, provider: &mut C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.recompute_all_fields(provider);
    }

    pub fn on_field_changed<C>(&mut self, provider: &mut C, changed_field: usize)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.on_field_changed(provider, changed_field);
    }

    pub fn effective_field_value(&self, field_index: usize) -> String {
        self.editor.effective_field_value(field_index)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use super::TextAreaEventOutcome;
    use super::TextAreaState;
    use crate::textarea::provider::TextAreaProvider;

    #[test]
    fn paste_splits_lines() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("ab");
        textarea.enter_edit_mode();
        textarea.set_cursor_position(2);

        textarea.paste("c\r\nd\nef");

        assert_eq!(textarea.text(), "abc\nd\nef");
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn input_reports_handled_and_ignored() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("");

        // A printable character is consumed.
        let out = textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Handled);
        assert_eq!(textarea.text(), "x");

        // Enter inserts a newline and is handled (never "submitted").
        let out = textarea.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Handled);
        assert_eq!(textarea.text(), "x\n");

        // An unrecognized key is ignored so a host can react to it.
        let out = textarea.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Ignored);
    }
}
