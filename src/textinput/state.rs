use std::ops::{Deref, DerefMut};

#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(feature = "cursor-style")]
use std::io;

#[cfg(feature = "cursor-style")]
use crate::{canvas::modes::AppMode, CursorManager};
use crate::editor::TextForm;
#[cfg(feature = "gui")]
use crate::gui_utils::{
    compute_h_scroll_with_padding, display_cols_up_to, display_width, RIGHT_PAD,
};
use crate::textinput::provider::{TextInputDataProvider, TextInputProvider};

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputEventOutcome {
    Ignored,
    Handled,
    Submitted,
}

/// Single-line text input widget state.
///
/// Wraps a single-field [`TextForm`]. Editing, cursor, and movement methods
/// from the form are available directly, and the form can be reached explicitly
/// via [`TextInputState::form`] / [`TextInputState::form_mut`].
/// With the `validation` and `computed` features enabled, the corresponding
/// helper methods are re-exposed as inherent methods on this type.
pub struct TextInputState<P: TextInputDataProvider = TextInputProvider> {
    pub(crate) form: TextForm<P>,
    pub(crate) placeholder: Option<String>,
    pub(crate) suggestion_suffix: Option<String>,
    pub(crate) overflow_indicator: char,
    pub(crate) h_scroll: u16,
    #[cfg(feature = "gui")]
    pub(crate) edited_this_frame: bool,
}

impl<P: TextInputDataProvider + Default> Default for TextInputState<P> {
    fn default() -> Self {
        Self {
            form: TextForm::new(P::default()),
            placeholder: None,
            suggestion_suffix: None,
            overflow_indicator: '$',
            h_scroll: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }
}

impl<P: TextInputDataProvider> Deref for TextInputState<P> {
    type Target = TextForm<P>;

    fn deref(&self) -> &Self::Target {
        &self.form
    }
}

impl<P: TextInputDataProvider> DerefMut for TextInputState<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.form
    }
}

impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn with_provider(provider: P) -> Self {
        Self {
            form: TextForm::new(provider),
            placeholder: None,
            suggestion_suffix: None,
            overflow_indicator: '$',
            h_scroll: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }

    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self::with_provider(P::from_text(text.into()))
    }

    pub fn text(&self) -> String {
        self.form.data_provider().to_text()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.form.data_provider_mut().set_text(text.into());
        self.form.ui_state.current_field = 0;
        self.form
            .set_cursor_raw(self.current_text().chars().count());
        self.clear_suggestion_suffix();
        self.h_scroll = 0;
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
    }

    pub fn suggestion_suffix(&self) -> Option<&str> {
        self.suggestion_suffix.as_deref()
    }

    pub fn set_suggestion_suffix<S: Into<String>>(&mut self, suffix: S) {
        let suffix = suffix.into();
        self.suggestion_suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        };
    }

    pub fn clear_suggestion_suffix(&mut self) {
        self.suggestion_suffix = None;
    }

    pub fn accept_suggestion_suffix(&mut self) -> TextInputEventOutcome {
        let Some(suffix) = self.suggestion_suffix.clone() else {
            return TextInputEventOutcome::Ignored;
        };

        if suffix.is_empty() {
            self.clear_suggestion_suffix();
            return TextInputEventOutcome::Ignored;
        }

        self.enter_edit_mode();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        self.clear_suggestion_suffix();
        let _ = self.insert_text(&suffix);
        TextInputEventOutcome::Handled
    }

    pub fn set_overflow_indicator(&mut self, ch: char) {
        self.overflow_indicator = ch;
    }

    pub fn paste(&mut self, text: &str) -> TextInputEventOutcome {
        let filtered: String = text
            .chars()
            .filter(|&ch| ch != '\n' && ch != '\r')
            .collect();

        if filtered.is_empty() {
            return TextInputEventOutcome::Ignored;
        }

        self.enter_edit_mode();
        self.clear_suggestion_suffix();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        let _ = self.insert_text(&filtered);
        TextInputEventOutcome::Handled
    }

    /// Update terminal cursor style for this single-line input.
    ///
    /// Text input is treated as insert-style editing, so this reuses the
    /// Canvas cursor policy for `AppMode::Ins`.
    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(AppMode::Ins)
    }

    #[cfg(not(feature = "cursor-style"))]
    pub fn update_cursor_style(&self) -> std::io::Result<()> {
        Ok(())
    }

    // TODO: Replace direct crossterm event coupling with a backend-agnostic
    // input abstraction so terminal input backends can be swapped cleanly.
    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> TextInputEventOutcome {
        match event {
            Event::Key(key) => self.input(key),
            Event::Paste(text) => self.paste(&text),
            _ => TextInputEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "crossterm")]
    pub fn input(&mut self, key: KeyEvent) -> TextInputEventOutcome {
        if key.kind != KeyEventKind::Press {
            return TextInputEventOutcome::Ignored;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => TextInputEventOutcome::Submitted,
            (KeyCode::Tab, KeyModifiers::NONE) => self.accept_suggestion_suffix(),
            (KeyCode::Backspace, _) => {
                self.clear_suggestion_suffix();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                let _ = self.delete_backward();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Delete, _) => {
                self.clear_suggestion_suffix();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                let _ = self.delete_forward();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Left, _) => {
                let _ = self.move_left();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Right, _) => {
                let _ = self.move_right();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Up, _) | (KeyCode::Down, _) => TextInputEventOutcome::Ignored,
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_line_start();
                TextInputEventOutcome::Handled
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_line_end();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                self.move_word_prev();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                self.move_word_next();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Char('e'), KeyModifiers::ALT) => {
                self.move_word_end();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Char(c), m) if m.is_empty() => {
                self.enter_edit_mode();
                self.clear_suggestion_suffix();
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                let _ = self.insert_char(c);
                TextInputEventOutcome::Handled
            }
            _ => TextInputEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        let cursor_cols = self.current_cursor_cols();
        let left_cols = if self.h_scroll > 0 { 1 } else { 0 };

        let mut x_off_visible = cursor_cols
            .saturating_sub(self.h_scroll)
            .saturating_add(left_cols);

        let limit = inner.width.saturating_sub(1 + RIGHT_PAD);
        if x_off_visible > limit {
            x_off_visible = limit;
        }

        (inner.x.saturating_add(x_off_visible), inner.y)
    }

    #[cfg(feature = "gui")]
    pub(crate) fn ensure_visible(&mut self, area: Rect, block: Option<&Block<'_>>) {
        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        if inner.width == 0 {
            return;
        }

        let total_cols = display_width(&self.current_display_text_for_render());
        if total_cols <= inner.width {
            self.h_scroll = 0;
            return;
        }

        let cursor_cols = self.current_cursor_cols();
        let (target_h, _) = compute_h_scroll_with_padding(cursor_cols, inner.width);

        if target_h > self.h_scroll {
            self.h_scroll = target_h;
        } else if cursor_cols < self.h_scroll {
            self.h_scroll = cursor_cols;
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn take_edited_flag(&mut self) -> bool {
        let v = self.edited_this_frame;
        self.edited_this_frame = false;
        v
    }

    #[cfg(feature = "gui")]
    pub(crate) fn current_display_text_for_render(&self) -> String {
        #[cfg(feature = "validation")]
        {
            self.current_display_text()
        }

        #[cfg(not(feature = "validation"))]
        {
            self.current_text().to_string()
        }
    }

    #[cfg(feature = "gui")]
    fn current_cursor_cols(&self) -> u16 {
        let text = self.current_display_text_for_render();
        display_cols_up_to(&text, self.display_cursor_position())
    }
}

impl<P: TextInputDataProvider> TextInputState<P> {
    /// Borrow the underlying [`TextForm`].
    pub fn form(&self) -> &TextForm<P> {
        &self.form
    }

    /// Mutably borrow the underlying [`TextForm`].
    pub fn form_mut(&mut self) -> &mut TextForm<P> {
        &mut self.form
    }

    /// Move to the start of the next word.
    pub fn move_word_next(&mut self) {
        self.form.move_word_next();
    }

    /// Move to the start of the previous word.
    pub fn move_word_prev(&mut self) {
        self.form.move_word_prev();
    }

    /// Move to the end of the current/next word.
    pub fn move_word_end(&mut self) {
        self.form.move_word_end();
    }

    /// Move to the end of the previous word.
    pub fn move_word_end_prev(&mut self) {
        self.form.move_word_end_prev();
    }

    /// Move to the start of the next WORD (whitespace-delimited, vim `W`).
    pub fn move_big_word_next(&mut self) {
        self.form.move_big_word_next();
    }

    /// Move to the start of the previous WORD (vim `B`).
    pub fn move_big_word_prev(&mut self) {
        self.form.move_big_word_prev();
    }

    /// Move to the end of the current/next WORD (vim `E`).
    pub fn move_big_word_end(&mut self) {
        self.form.move_big_word_end();
    }

    /// Move to the end of the previous WORD (vim `gE`).
    pub fn move_big_word_end_prev(&mut self) {
        self.form.move_big_word_end_prev();
    }

    /// Enter edit mode with the cursor positioned for append (vim `a`).
    pub fn enter_append_mode(&mut self) {
        self.form.enter_append_mode();
    }

    /// The current field's display text (mask/formatter-aware when the
    /// `validation` feature is enabled; otherwise the raw text).
    #[cfg(feature = "validation")]
    pub fn current_display_text(&self) -> String {
        self.form.current_display_text()
    }

    /// The current field's text (raw; no validation feature for masking).
    #[cfg(not(feature = "validation"))]
    pub fn current_display_text(&self) -> String {
        self.form.current_text().to_string()
    }

    /// Cursor position in display coordinates (accounts for a display mask).
    pub fn display_cursor_position(&self) -> usize {
        self.form.display_cursor_position()
    }
}

/// Validation helpers, re-exposed from the underlying [`TextForm`] so they are
/// part of `TextInputState`'s own public API rather than only reachable through
/// `Deref`.
#[cfg(feature = "validation")]
impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn set_validation_enabled(&mut self, enabled: bool) {
        self.form.set_validation_enabled(enabled);
    }

    pub fn is_validation_enabled(&self) -> bool {
        self.form.is_validation_enabled()
    }

    pub fn set_field_validation(
        &mut self,
        field_index: usize,
        config: crate::validation::ValidationConfig,
    ) {
        self.form.set_field_validation(field_index, config);
    }

    pub fn remove_field_validation(&mut self, field_index: usize) {
        self.form.remove_field_validation(field_index);
    }

    pub fn validate_current_field(&mut self) -> crate::validation::ValidationResult {
        self.form.validate_current_field()
    }

    pub fn validate_field(
        &mut self,
        field_index: usize,
    ) -> Option<crate::validation::ValidationResult> {
        self.form.validate_field(field_index)
    }

    pub fn clear_validation_results(&mut self) {
        self.form.clear_validation_results();
    }

    pub fn validation_summary(&self) -> crate::validation::ValidationSummary {
        self.form.validation_summary()
    }

    pub fn can_switch_fields(&self) -> bool {
        self.form.can_switch_fields()
    }

    pub fn field_switch_block_reason(&self) -> Option<String> {
        self.form.field_switch_block_reason()
    }

    pub fn last_switch_block(&self) -> Option<&str> {
        self.form.last_switch_block()
    }

    pub fn current_limits_status_text(&self) -> Option<String> {
        self.form.current_limits_status_text()
    }

    pub fn current_formatter_warning(&self) -> Option<String> {
        self.form.current_formatter_warning()
    }

    pub fn external_validation_of(
        &self,
        field_index: usize,
    ) -> crate::validation::ExternalValidationState {
        self.form.external_validation_of(field_index)
    }

    pub fn clear_all_external_validation(&mut self) {
        self.form.clear_all_external_validation();
    }

    pub fn clear_external_validation(&mut self, field_index: usize) {
        self.form.clear_external_validation(field_index);
    }

    pub fn set_external_validation(
        &mut self,
        field_index: usize,
        state: crate::validation::ExternalValidationState,
    ) {
        self.form.set_external_validation(field_index, state);
    }

    pub fn set_external_validation_callback<F>(&mut self, callback: F)
    where
        F: FnMut(usize, &str) -> crate::validation::ExternalValidationState + Send + Sync + 'static,
    {
        self.form.set_external_validation_callback(callback);
    }
}

/// Computed-field helpers, re-exposed from the underlying [`TextForm`].
#[cfg(feature = "computed")]
impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn register_computed_provider<C>(&mut self, provider: &C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.form.register_computed_provider(provider);
    }

    pub fn set_computed_provider<C>(&mut self, provider: C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.form.set_computed_provider(provider);
    }

    pub fn recompute_fields<C>(&mut self, provider: &mut C, field_indices: &[usize])
    where
        C: crate::computed::ComputedProvider,
    {
        self.form.recompute_fields(provider, field_indices);
    }

    pub fn recompute_all_fields<C>(&mut self, provider: &mut C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.form.recompute_all_fields(provider);
    }

    pub fn on_field_changed<C>(&mut self, provider: &mut C, changed_field: usize)
    where
        C: crate::computed::ComputedProvider,
    {
        self.form.on_field_changed(provider, changed_field);
    }

    pub fn effective_field_value(&self, field_index: usize) -> String {
        self.form.effective_field_value(field_index)
    }
}

/// Undo/redo, re-exposed from the underlying [`TextForm`].
impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn undo(&mut self) -> bool {
        self.form.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.form.redo()
    }

    pub fn can_undo(&self) -> bool {
        self.form.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.form.can_redo()
    }

    pub fn clear_history(&mut self) {
        self.form.clear_history();
    }

    pub fn set_history_limit(&mut self, limit: usize) {
        self.form.set_history_limit(limit);
    }
}

/// Dropdown suggestions, re-exposed from the underlying [`TextForm`] so that
/// `TextInput`, `TextArea`, and `TextForm` all share one suggestions
/// mechanism. Render the dropdown with
/// `canvas::suggestions::render::render_suggestions_dropdown(.., self.form())`.
///
/// This is distinct from the lightweight inline-suffix completion
/// ([`TextInputState::set_suggestion_suffix`]), which `TextInput` also offers.
#[cfg(feature = "suggestions")]
impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn open_suggestions(&mut self, field_index: usize) {
        self.form.open_suggestions(field_index);
    }

    pub fn trigger_suggestions(&mut self) -> Option<(usize, String)> {
        self.form.trigger_suggestions()
    }

    pub fn apply_suggestions(&mut self, items: Vec<crate::SuggestionItem>) {
        self.form.apply_suggestions(items);
    }

    pub fn update_suggestions(&mut self, items: Vec<crate::SuggestionItem>) {
        self.form.update_suggestions(items);
    }

    pub fn dismiss_suggestions(&mut self) {
        self.form.dismiss_suggestions();
    }

    pub fn cancel_suggestions(&mut self) {
        self.form.cancel_suggestions();
    }

    pub fn suggestions_next(&mut self) {
        self.form.suggestions_next();
    }

    pub fn suggestions_prev(&mut self) {
        self.form.suggestions_prev();
    }

    pub fn apply_suggestion(&mut self) -> Option<String> {
        self.form.apply_suggestion()
    }

    pub fn is_suggestions_active(&self) -> bool {
        self.form.is_suggestions_active()
    }

    pub fn is_suggestions_loading(&self) -> bool {
        self.form.ui_state().is_suggestions_loading()
    }

    pub fn dropdown_suggestions(&self) -> &[crate::SuggestionItem] {
        self.form.suggestions()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use crossterm::event::Event;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{TextInputEventOutcome, TextInputState};
    use crate::textinput::provider::TextInputProvider;

    #[test]
    fn enter_submits_without_mutating_text() {
        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        let outcome = input.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(outcome, TextInputEventOutcome::Submitted);
        assert_eq!(input.text(), "hello");
    }

    #[test]
    fn vertical_arrows_are_ignored() {
        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        let outcome = input.input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert_eq!(outcome, TextInputEventOutcome::Ignored);
        assert_eq!(input.current_field(), 0);
    }

    #[test]
    fn paste_filters_line_breaks_for_single_line_input() {
        let mut input = TextInputState::<TextInputProvider>::from_text("ab");
        input.enter_edit_mode();
        input.set_cursor_position(2);

        let outcome = input.paste("c\r\nd\nef");

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(input.text(), "abcdef");
    }

    #[test]
    fn tab_accepts_inline_suggestion_suffix() {
        let mut input = TextInputState::<TextInputProvider>::from_text("ad");
        input.enter_edit_mode();
        input.set_cursor_position(2);
        input.set_suggestion_suffix("min");

        let outcome = input.input(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(input.text(), "admin");
        assert_eq!(input.suggestion_suffix(), None);
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn handle_event_routes_paste_events() {
        let mut input = TextInputState::<TextInputProvider>::from_text("hi");
        input.enter_edit_mode();
        input.set_cursor_position(2);

        let outcome = input.handle_event(Event::Paste(" there".to_string()));

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(input.text(), "hi there");
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut input = TextInputState::<TextInputProvider>::from_text("");
        input.enter_edit_mode();
        let _ = input.insert_text("abc"); // one coalesced Insert run

        assert_eq!(input.text(), "abc");
        assert!(input.can_undo());
        assert!(input.undo());
        assert_eq!(input.text(), "");
        assert!(input.can_redo());
        assert!(input.redo());
        assert_eq!(input.text(), "abc");
        assert!(!input.redo());
    }

    #[test]
    fn undo_separates_insert_and_delete_runs() {
        let mut input = TextInputState::<TextInputProvider>::from_text("");
        input.enter_edit_mode();
        let _ = input.insert_text("ab"); // Insert run
        let _ = input.delete_backward(); // Delete run -> separate step
        assert_eq!(input.text(), "a");

        assert!(input.undo()); // undo the delete
        assert_eq!(input.text(), "ab");
        assert!(input.undo()); // undo the insert run
        assert_eq!(input.text(), "");
        assert!(!input.undo());
    }

    #[test]
    fn history_limit_drops_oldest() {
        let mut input = TextInputState::<TextInputProvider>::from_text("");
        input.set_history_limit(2);

        // Three non-coalescing edits; only the last two checkpoints are kept.
        input.set_current_field_value("one".to_string());
        input.set_current_field_value("two".to_string());
        input.set_current_field_value("three".to_string());

        assert!(input.undo());
        assert_eq!(input.text(), "two");
        assert!(input.undo());
        assert_eq!(input.text(), "one");
        // The checkpoint for the empty initial state was dropped by the limit.
        assert!(!input.undo());
    }

    #[test]
    fn cursor_move_breaks_coalescing() {
        let mut input = TextInputState::<TextInputProvider>::from_text("");
        input.enter_edit_mode();
        let _ = input.insert_text("ab");
        let _ = input.move_left(); // ends the typing run
        let _ = input.insert_text("X"); // new run, inserted before 'b'
        assert_eq!(input.text(), "aXb");

        assert!(input.undo()); // undo "X"
        assert_eq!(input.text(), "ab");
        assert!(input.undo()); // undo "ab"
        assert_eq!(input.text(), "");
    }
}
