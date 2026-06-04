#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::textarea::{TextAreaDataProvider, TextAreaEventOutcome, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
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
}

#[cfg(feature = "crossterm")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn handle_event(&mut self, event: Event) -> TextAreaEventOutcome {
        match event {
            Event::Key(key) => self.input(key),
            Event::Paste(text) => self.paste(&text),
            _ => TextAreaEventOutcome::Ignored,
        }
    }

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
                self.insert_tab_spaces();
            }

            _ => return TextAreaEventOutcome::Ignored,
        }

        TextAreaEventOutcome::Handled
    }
}
