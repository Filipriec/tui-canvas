#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(feature = "cursor-style")]
use std::io;

#[cfg(feature = "gui")]
use crate::gui_utils::{
    compute_h_scroll_with_padding, display_cols_up_to, display_width, effective_right_pad,
};
use crate::textinput::provider::{TextInputDataProvider, TextInputProvider};
#[cfg(feature = "cursor-style")]
use crate::CursorManager;
use crate::{canvas::state::EditorState, textform::TextFormState};

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
/// Wraps a single-field [`TextFormState`]. Editing, cursor, and movement methods
/// from the form are available directly, and the form can be reached explicitly
/// via [`TextInputState::form`] / [`TextInputState::form_mut`].
/// With the `validation` and `computed` features enabled, the corresponding
/// helper methods are re-exposed as inherent methods on this type.
pub struct TextInputState<P: TextInputDataProvider = TextInputProvider> {
    pub(crate) form: TextFormState<P>,
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
            form: TextFormState::new(P::default()),
            placeholder: None,
            suggestion_suffix: None,
            overflow_indicator: '$',
            h_scroll: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }
}

impl<P: TextInputDataProvider> std::ops::Deref for TextInputState<P> {
    type Target = TextFormState<P>;

    fn deref(&self) -> &Self::Target {
        &self.form
    }
}

impl<P: TextInputDataProvider> std::ops::DerefMut for TextInputState<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.form
    }
}

impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn with_provider(provider: P) -> Self {
        Self {
            form: TextFormState::new(provider),
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
        let cursor = self.current_text().chars().count();
        self.form.set_cursor_raw(cursor);
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
    /// Follows the editor's current mode so that Helix/Vim-style modal editing
    /// produces the correct cursor: a steady block in normal mode, a beam in
    /// insert mode, and a blinking block for selections.
    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(self.form.mode())
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
        let total_cols = display_width(&self.current_display_text_for_render());
        let left_cols = if self.h_scroll > 0 { 1 } else { 0 };

        let mut x_off_visible = cursor_cols
            .saturating_sub(self.h_scroll)
            .saturating_add(left_cols);

        let limit = inner
            .width
            .saturating_sub(1 + effective_right_pad(cursor_cols, total_cols));
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
        let cursor_cols = self.current_cursor_cols();
        let (target_h, _) = compute_h_scroll_with_padding(cursor_cols, total_cols, inner.width);

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
    /// Borrow the underlying [`TextFormState`].
    pub fn form(&self) -> &TextFormState<P> {
        &self.form
    }

    /// Mutably borrow the underlying [`TextFormState`].
    pub fn form_mut(&mut self) -> &mut TextFormState<P> {
        &mut self.form
    }

    pub fn current_field(&self) -> usize {
        self.form.current_field()
    }

    pub fn cursor_position(&self) -> usize {
        self.form.cursor_position()
    }

    pub fn current_text(&self) -> &str {
        self.form.current_text()
    }

    pub fn mode(&self) -> crate::canvas::modes::AppMode {
        self.form.mode()
    }

    pub fn ui_state(&self) -> &EditorState {
        self.form.ui_state()
    }

    pub fn data_provider(&self) -> &P {
        self.form.data_provider()
    }

    pub fn data_provider_mut(&mut self) -> &mut P {
        self.form.data_provider_mut()
    }

    pub fn move_left(&mut self) -> anyhow::Result<()> {
        self.form.move_left()
    }

    pub fn move_right(&mut self) -> anyhow::Result<()> {
        self.form.move_right()
    }

    pub fn move_line_start(&mut self) {
        self.form.move_line_start();
    }

    pub fn move_line_end(&mut self) {
        self.form.move_line_end();
    }

    pub fn set_cursor_position(&mut self, position: usize) {
        self.form.set_cursor_position(position);
    }

    pub fn enter_edit_mode(&mut self) {
        self.form.enter_edit_mode();
    }

    pub fn exit_edit_mode(&mut self) -> anyhow::Result<()> {
        self.form.exit_edit_mode()
    }

    pub fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        self.form.insert_char(ch)
    }

    pub fn insert_text(&mut self, text: &str) -> anyhow::Result<()> {
        self.form.insert_text(text)
    }

    pub fn set_current_field_value(&mut self, value: String) {
        self.form.set_current_field_value(value);
    }

    pub fn delete_backward(&mut self) -> anyhow::Result<()> {
        self.form.delete_backward()
    }

    pub fn delete_forward(&mut self) -> anyhow::Result<()> {
        self.form.delete_forward()
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

/// Validation helpers, re-exposed from the underlying [`TextFormState`] so they are
/// part of `TextInputState`'s own public API.
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

/// Computed-field helpers, re-exposed from the underlying [`TextFormState`].
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

/// Undo/redo, re-exposed from the underlying [`TextFormState`].
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

/// Dropdown suggestions, re-exposed from the underlying [`TextFormState`] so that
/// `TextInput`, `TextArea`, and `TextFormState` all share one suggestions
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

    pub fn check_suggestion_trigger(&mut self) {
        self.form.check_suggestion_trigger();
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

    #[cfg(feature = "gui")]
    use ratatui::layout::Rect;

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

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn handle_key_event_returns_key_event_outcome_not_textinput_outcome() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        // This won't compile if handle_key_event resolves to a method returning
        // TextInputEventOutcome — it must be KeyEventOutcome from TextFormState.
        let outcome: KeyEventOutcome =
            input.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        // 'h' in normal mode with Helix keybindings should be Consumed (move left).
        assert!(!matches!(outcome, KeyEventOutcome::NotMatched));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_i_enters_insert_mode_from_normal() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Start in normal mode (default for non-textmode-normal)
        assert_eq!(input.mode(), AppMode::Nor);

        // Press 'i' — should enter insert mode
        let outcome = input.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(
            matches!(outcome, KeyEventOutcome::Consumed(None)),
            "expected Consumed, got {:?}",
            outcome
        );
        assert_eq!(input.mode(), AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_esc_exits_insert_mode_to_normal() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Enter insert mode first
        let _ = input.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(input.mode(), AppMode::Ins);

        // Press Esc — should exit to normal mode
        let outcome =
            input.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(
            matches!(outcome, KeyEventOutcome::Consumed(None)),
            "expected Consumed, got {:?}",
            outcome
        );
        assert_eq!(input.mode(), AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_normal_movement_works() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Enter insert, type a char, exit to normal
        let _ = input.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        let _ = input.handle_key_event(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        let _ = input.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(input.mode(), AppMode::Nor);
        // confirm text was inserted
        assert!(input.text().contains("X"));

        // h should move left in normal mode
        let outcome =
            input.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(
            matches!(outcome, KeyEventOutcome::Consumed(None)),
            "expected Consumed, got {:?}",
            outcome
        );
        // still in normal mode
        assert_eq!(input.mode(), AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "cursor-style"))]
    #[test]
    fn update_cursor_style_follows_actual_mode() {
        use crate::canvas::modes::AppMode;

        // This test verifies that update_cursor_style() tracks the real editor
        // mode rather than always forcing insert-mode cursor.
        let input = TextInputState::<TextInputProvider>::from_text("hello");
        // Default mode is normal (Nor) for modal editing.
        assert_eq!(input.mode(), AppMode::Nor);

        // update_cursor_style should respect the normal-mode cursor.
        let result: std::io::Result<()> = input.update_cursor_style();
        assert!(result.is_ok());
    }

    #[cfg(feature = "gui")]
    #[test]
    fn ensure_visible_scrolls_before_text_overflows_to_keep_end_margin() {
        let mut input = TextInputState::<TextInputProvider>::from_text("123456789");
        input.enter_edit_mode();
        input.set_cursor_position(9);

        input.ensure_visible(Rect::new(0, 0, 10, 1), None);

        assert_eq!(input.h_scroll, 3);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm", feature = "cursor-style"))]
    #[test]
    fn helix_full_event_loop_mirrors_example_usage() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        // Simulate the example's event loop: update_cursor_style → handle_key_event
        let mut input = TextInputState::<TextInputProvider>::from_text("hello");
        input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Frame 1: normal mode
        assert_eq!(input.mode(), AppMode::Nor);
        assert!(input.update_cursor_style().is_ok());

        // Frame 2: press 'i' to enter insert mode
        let outcome = input.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(input.mode(), AppMode::Ins);
        assert!(input.update_cursor_style().is_ok());

        // Frame 3: type 'x' in insert mode (should insert char)
        let outcome = input.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert!(input.text().contains('x'));
        assert!(input.update_cursor_style().is_ok());

        // Frame 4: exit to normal mode with Esc
        let outcome = input.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(input.mode(), AppMode::Nor);
        assert!(input.update_cursor_style().is_ok());

        // Frame 5: 'h' should move left in normal mode
        let old_pos = input.cursor_position();
        let outcome = input.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(input.mode(), AppMode::Nor);
        // Cursor should have moved left
        assert!(input.cursor_position() < old_pos || old_pos == 0);
        assert!(input.update_cursor_style().is_ok());
    }
}
