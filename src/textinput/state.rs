use std::ops::{Deref, DerefMut};

#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(feature = "cursor-style")]
use std::io;

#[cfg(feature = "cursor-style")]
use crate::canvas::{CursorManager, modes::AppMode};
use crate::editor::FormEditor;
#[cfg(feature = "gui")]
use crate::gui_utils::{
    compute_h_scroll_with_padding, display_cols_up_to, display_width,
    RIGHT_PAD,
};
use crate::textinput::provider::{TextInputDataProvider, TextInputProvider};

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

pub type TextInputEditor<P = TextInputProvider> = FormEditor<P>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputEventOutcome {
    Ignored,
    Handled,
    Submitted,
}

pub struct TextInputState<P: TextInputDataProvider = TextInputProvider> {
    pub(crate) editor: TextInputEditor<P>,
    pub(crate) placeholder: Option<String>,
    pub(crate) overflow_indicator: char,
    pub(crate) h_scroll: u16,
    #[cfg(feature = "gui")]
    pub(crate) edited_this_frame: bool,
}

impl<P: TextInputDataProvider + Default> Default for TextInputState<P> {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(P::default()),
            placeholder: None,
            overflow_indicator: '$',
            h_scroll: 0,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
        }
    }
}

impl<P: TextInputDataProvider> Deref for TextInputState<P> {
    type Target = TextInputEditor<P>;

    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl<P: TextInputDataProvider> DerefMut for TextInputState<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

impl<P: TextInputDataProvider> TextInputState<P> {
    pub fn with_provider(provider: P) -> Self {
        Self {
            editor: FormEditor::new(provider),
            placeholder: None,
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
        self.editor.data_provider().to_text()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.editor.data_provider_mut().set_text(text.into());
        self.editor.ui_state.current_field = 0;
        self.editor.set_cursor_raw(self.current_text().chars().count());
        self.h_scroll = 0;
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
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
    /// Canvas cursor policy for `AppMode::Edit`.
    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(AppMode::Edit)
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
            (KeyCode::Backspace, _) => {
                #[cfg(feature = "gui")]
                {
                    self.edited_this_frame = true;
                }
                let _ = self.delete_backward();
                TextInputEventOutcome::Handled
            }
            (KeyCode::Delete, _) => {
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
            (KeyCode::Up, _) | (KeyCode::Down, _) => {
                TextInputEventOutcome::Ignored
            }
            (KeyCode::Home, _)
            | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_line_start();
                TextInputEventOutcome::Handled
            }
            (KeyCode::End, _)
            | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
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
        let inner = if let Some(b) = block { b.inner(area) } else { area };
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
        let inner = if let Some(b) = block { b.inner(area) } else { area };
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

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use crossterm::event::Event;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{TextInputEventOutcome, TextInputState};
    use crate::textinput::provider::TextInputProvider;

    #[test]
    fn enter_submits_without_mutating_text() {
        let mut input =
            TextInputState::<TextInputProvider>::from_text("hello");
        let outcome = input.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(outcome, TextInputEventOutcome::Submitted);
        assert_eq!(input.text(), "hello");
    }

    #[test]
    fn vertical_arrows_are_ignored() {
        let mut input =
            TextInputState::<TextInputProvider>::from_text("hello");
        let outcome = input.input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert_eq!(outcome, TextInputEventOutcome::Ignored);
        assert_eq!(input.current_field(), 0);
    }

    #[test]
    fn paste_filters_line_breaks_for_single_line_input() {
        let mut input =
            TextInputState::<TextInputProvider>::from_text("ab");
        input.enter_edit_mode();
        input.set_cursor_position(2);

        let outcome = input.paste("c\r\nd\nef");

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(input.text(), "abcdef");
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn handle_event_routes_paste_events() {
        let mut input =
            TextInputState::<TextInputProvider>::from_text("hi");
        input.enter_edit_mode();
        input.set_cursor_position(2);

        let outcome = input.handle_event(Event::Paste(" there".to_string()));

        assert_eq!(outcome, TextInputEventOutcome::Handled);
        assert_eq!(input.text(), "hi there");
    }
}
