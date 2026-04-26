#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::editor::FormEditor;
use crate::DataProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormInputEventOutcome {
    Ignored,
    Handled,
    Submitted,
}

impl<D: DataProvider> FormEditor<D> {
    pub fn paste(&mut self, text: &str) -> FormInputEventOutcome {
        let filtered: String = text
            .chars()
            .filter(|&ch| ch != '\n' && ch != '\r')
            .collect();

        if filtered.is_empty() {
            return FormInputEventOutcome::Ignored;
        }

        self.enter_edit_mode();
        let _ = self.insert_text(&filtered);
        FormInputEventOutcome::Handled
    }

    // TODO: Replace direct crossterm event coupling with a backend-agnostic
    // input abstraction so terminal input backends can be swapped cleanly.
    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> FormInputEventOutcome {
        match event {
            Event::Key(key) => self.input(key),
            Event::Paste(text) => self.paste(&text),
            _ => FormInputEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "crossterm")]
    pub fn input(&mut self, key: KeyEvent) -> FormInputEventOutcome {
        if key.kind != KeyEventKind::Press {
            return FormInputEventOutcome::Ignored;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => {
                let last =
                    self.data_provider().field_count().saturating_sub(1);
                if self.current_field() >= last {
                    FormInputEventOutcome::Submitted
                } else {
                    let _ = self.next_field();
                    FormInputEventOutcome::Handled
                }
            }
            (KeyCode::Tab, _) => {
                let _ = self.next_field();
                FormInputEventOutcome::Handled
            }
            (KeyCode::BackTab, _) => {
                let _ = self.prev_field();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Backspace, _) => {
                let _ = self.delete_backward();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Delete, _) => {
                let _ = self.delete_forward();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
                self.move_word_prev();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
                self.move_word_next();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Left, _) => {
                let _ = self.move_left();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Right, _) => {
                let _ = self.move_right();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Up, _) => {
                let _ = self.move_up();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Down, _) => {
                let _ = self.move_down();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Home, _)
            | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_line_start();
                FormInputEventOutcome::Handled
            }
            (KeyCode::End, _)
            | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_line_end();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                self.move_word_prev();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                self.move_word_next();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Char('e'), KeyModifiers::ALT) => {
                self.move_word_end();
                FormInputEventOutcome::Handled
            }
            (KeyCode::Esc, _) => {
                if self.mode() == crate::canvas::modes::AppMode::Edit {
                    let _ = self.exit_edit_mode();
                    FormInputEventOutcome::Handled
                } else {
                    FormInputEventOutcome::Ignored
                }
            }
            (KeyCode::Char(c), m)
                if !m.contains(KeyModifiers::CONTROL)
                    && !m.contains(KeyModifiers::ALT) =>
            {
                self.enter_edit_mode();
                let _ = self.insert_char(c);
                FormInputEventOutcome::Handled
            }
            _ => FormInputEventOutcome::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use super::FormInputEventOutcome;
    use crate::{DataProvider, FormEditor};

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

    #[test]
    fn paste_filters_line_breaks_in_form_fields() {
        let mut editor = FormEditor::new(TestProvider::default());
        editor.enter_edit_mode();

        let outcome = editor.paste("ab\r\ncd");

        assert_eq!(outcome, FormInputEventOutcome::Handled);
        assert_eq!(editor.current_text(), "abcd");
    }

    #[test]
    fn enter_submits_on_last_field() {
        let mut editor = FormEditor::new(TestProvider::default());
        let _ = editor.next_field();

        let outcome = editor
            .input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(outcome, FormInputEventOutcome::Submitted);
        assert_eq!(editor.current_field(), 1);
    }

    #[test]
    fn handle_event_routes_paste() {
        let mut editor = FormEditor::new(TestProvider::default());
        editor.enter_edit_mode();

        let outcome = editor.handle_event(Event::Paste("hello".to_string()));

        assert_eq!(outcome, FormInputEventOutcome::Handled);
        assert_eq!(editor.current_text(), "hello");
    }
}
