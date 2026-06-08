#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use std::ops::{Deref, DerefMut};

use crate::{editor::EditorCore, DataProvider};

#[cfg(feature = "keybindings")]
use crate::{
    canvas::actions::ActionResult,
    editor::product::{handle_product_key_event, KeybindingProduct},
    integration::focus_handoff::{key_outcome_for_vertical_navigation, BoundaryExit},
    keybindings::{CanvasKeyAction, CanvasKeyBindings, KeyEventOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormEventOutcome {
    Ignored,
    Handled,
    Submitted,
}

#[derive(Debug, Default)]
pub struct TextFormState<D: DataProvider> {
    pub(crate) core: EditorCore<D>,
}

impl<D: DataProvider> TextFormState<D> {
    pub fn new(data_provider: D) -> Self {
        Self {
            core: EditorCore::new(data_provider),
        }
    }

    pub fn with_provider(data_provider: D) -> Self {
        Self::new(data_provider)
    }

    pub fn core(&self) -> &EditorCore<D> {
        &self.core
    }

    pub fn core_mut(&mut self) -> &mut EditorCore<D> {
        &mut self.core
    }

    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> TextFormEventOutcome {
        match event {
            Event::Key(key) => self.input(key),
            Event::Paste(text) => self.paste(&text),
            _ => TextFormEventOutcome::Ignored,
        }
    }

    pub fn paste(&mut self, text: &str) -> TextFormEventOutcome {
        let filtered: String = text
            .chars()
            .filter(|&ch| ch != '\n' && ch != '\r')
            .collect();

        if filtered.is_empty() {
            return TextFormEventOutcome::Ignored;
        }

        self.core.enter_edit_mode();
        let _ = self.core.insert_text(&filtered);
        TextFormEventOutcome::Handled
    }

    #[cfg(feature = "crossterm")]
    pub fn input(&mut self, key: KeyEvent) -> TextFormEventOutcome {
        if key.kind != KeyEventKind::Press {
            return TextFormEventOutcome::Ignored;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => self.enter_next_field_or_submit(),
            (KeyCode::Tab, _) => {
                let _ = self.core.next_field();
                TextFormEventOutcome::Handled
            }
            (KeyCode::BackTab, _) => {
                let _ = self.core.prev_field();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Backspace, _) => {
                let _ = self.core.delete_backward();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Delete, _) => {
                let _ = self.core.delete_forward();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
                self.core.move_word_prev();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
                self.core.move_word_next();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Left, _) => {
                let _ = self.core.move_left();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Right, _) => {
                let _ = self.core.move_right();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Up, _) => {
                let _ = self.core.move_up();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Down, _) => {
                let _ = self.core.move_down();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.core.move_line_start();
                TextFormEventOutcome::Handled
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.core.move_line_end();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                self.core.move_word_prev();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                self.core.move_word_next();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Char('e'), KeyModifiers::ALT) => {
                self.core.move_word_end();
                TextFormEventOutcome::Handled
            }
            (KeyCode::Esc, _) => {
                if self.core.mode() == crate::canvas::modes::AppMode::Ins {
                    let _ = self.core.exit_edit_mode();
                    TextFormEventOutcome::Handled
                } else {
                    TextFormEventOutcome::Ignored
                }
            }
            (KeyCode::Char(c), m)
                if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) =>
            {
                self.core.enter_edit_mode();
                let _ = self.core.insert_char(c);
                TextFormEventOutcome::Handled
            }
            _ => TextFormEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "keybindings")]
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        handle_product_key_event(self, evt)
    }

    #[cfg(feature = "keybindings")]
    pub fn use_keybinding_preset(
        &mut self,
        preset: crate::keybindings::BuiltinCanvasKeybindingPreset,
    ) {
        self.core.set_keybinding_preset(preset);
    }

    #[cfg(feature = "keybindings")]
    pub fn set_keybindings(&mut self, keybindings: CanvasKeyBindings) {
        self.core.set_keybindings(keybindings);
    }

    pub fn clear_current_field(&mut self) {
        self.core.set_current_field_value(String::new());
    }

    pub fn change_current_field(&mut self) {
        self.clear_current_field();
        self.core.enter_edit_mode();
    }

    pub fn delete_to_field_end(&mut self) {
        let cursor = self.core.cursor_position();
        let kept: String = self.core.current_text().chars().take(cursor).collect();
        self.core.set_current_field_value(kept);
        self.core.set_cursor_position(cursor);
    }

    #[cfg(feature = "crossterm")]
    fn enter_next_field_or_submit(&mut self) -> TextFormEventOutcome {
        let last = self.core.data_provider().field_count().saturating_sub(1);
        if self.core.current_field() >= last {
            TextFormEventOutcome::Submitted
        } else {
            let _ = self.core.next_field();
            TextFormEventOutcome::Handled
        }
    }

    #[cfg(feature = "keybindings")]
    fn move_next_field_count(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.core.next_field();
        }
    }

    #[cfg(feature = "keybindings")]
    fn move_prev_field_count(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.core.prev_field();
        }
    }

    #[cfg(feature = "keybindings")]
    fn execute_canvas_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        let Some(canvas_action) = action.to_canvas_action() else {
            return KeyEventOutcome::NotMatched;
        };

        let boundary = match action {
            CanvasKeyAction::MoveUp | CanvasKeyAction::PrevField => Some(BoundaryExit::Top),
            CanvasKeyAction::MoveDown | CanvasKeyAction::NextField => Some(BoundaryExit::Bottom),
            _ => None,
        };
        let before_field = self.core.current_field();

        let mut result = ActionResult::Success;
        for _ in 0..count {
            result = self.core.execute(canvas_action.clone());
        }

        if let Some(boundary) = boundary {
            let moved = self.core.current_field() != before_field;
            return key_outcome_for_vertical_navigation(moved, boundary);
        }

        match result {
            ActionResult::Success => KeyEventOutcome::Consumed(None),
            ActionResult::Message(msg) | ActionResult::Error(msg) => {
                KeyEventOutcome::Consumed(Some(msg))
            }
        }
    }
}

impl<D: DataProvider> Deref for TextFormState<D> {
    type Target = EditorCore<D>;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl<D: DataProvider> DerefMut for TextFormState<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

#[cfg(feature = "keybindings")]
impl<D: DataProvider> KeybindingProduct for TextFormState<D> {
    type Provider = D;

    fn core(&self) -> &EditorCore<Self::Provider> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut EditorCore<Self::Provider> {
        &mut self.core
    }

    fn handle_insert_enter(&mut self) -> KeyEventOutcome {
        self.move_next_field_count(1);
        KeyEventOutcome::Consumed(None)
    }

    fn handle_insert_tab(&mut self) -> KeyEventOutcome {
        self.move_next_field_count(1);
        KeyEventOutcome::Consumed(None)
    }

    fn handle_plain_insert_char(&mut self, ch: char) -> KeyEventOutcome {
        self.core.enter_edit_mode();
        if self.core.insert_char(ch).is_ok() {
            KeyEventOutcome::Consumed(None)
        } else {
            KeyEventOutcome::NotMatched
        }
    }

    fn dispatch_product_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        match action {
            CanvasKeyAction::NextField => {
                self.move_next_field_count(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PrevField => {
                self.move_prev_field_count(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineBelow => {
                self.move_next_field_count(count);
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineAbove => {
                self.move_prev_field_count(count);
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineStart => {
                self.core.move_line_start();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineEnd => {
                self.core.move_line_end();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteLine => {
                for _ in 0..count {
                    self.clear_current_field();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteToLineEnd => {
                self.delete_to_field_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeLine => {
                self.change_current_field();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeToLineEnd => {
                self.delete_to_field_end();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            _ => self.execute_canvas_key_action(action, count),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::TextFormState;
    #[cfg(feature = "crossterm")]
    use super::TextFormEventOutcome;
    use crate::DataProvider;

    #[derive(Default)]
    struct TestProvider {
        fields: [String; 2],
    }

    impl DataProvider for TestProvider {
        fn field_count(&self) -> usize {
            2
        }

        fn field_name(&self, index: usize) -> &str {
            match index {
                0 => "first",
                1 => "second",
                _ => "",
            }
        }

        fn field_value(&self, index: usize) -> &str {
            self.fields.get(index).map(String::as_str).unwrap_or("")
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            if let Some(field) = self.fields.get_mut(index) {
                *field = value;
            }
        }
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn enter_moves_between_fields_then_submits() {
        let mut form = TextFormState::new(TestProvider::default());

        let first = form.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(first, TextFormEventOutcome::Handled);
        assert_eq!(form.current_field(), 1);

        let second = form.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(second, TextFormEventOutcome::Submitted);
        assert_eq!(form.current_field(), 1);
    }

    #[test]
    fn delete_line_clears_current_field_without_removing_it() {
        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "def".to_string()],
        });
        form.clear_current_field();

        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.current_text(), "");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn keybinding_enter_moves_to_next_field_without_splitting_rows() {
        use crate::keybindings::KeyEventOutcome;

        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "def".to_string()],
        });
        form.enter_edit_mode();

        let outcome = form.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(outcome, KeyEventOutcome::Consumed(None));
        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.current_field(), 1);
        assert_eq!(form.data_provider().field_value(0), "abc");
        assert_eq!(form.current_text(), "def");
    }
}
