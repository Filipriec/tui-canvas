use super::state::{
    CommandLineEventOutcome, CommandLineMode, CommandLineState,
};
use crate::TextInputEventOutcome;

#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(feature = "crossterm")]
fn is_plain_prompt_modifier(modifiers: KeyModifiers) -> bool {
    !modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT)
}

impl CommandLineState {
    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> CommandLineEventOutcome {
        match event {
            Event::Key(key) => self.input_key(key),
            Event::Paste(text) if self.is_active() => {
                let outcome = self.input.paste(&text);
                match outcome {
                    TextInputEventOutcome::Ignored => CommandLineEventOutcome::Ignored,
                    TextInputEventOutcome::Handled | TextInputEventOutcome::Submitted => {
                        CommandLineEventOutcome::Handled
                    }
                }
            }
            _ => CommandLineEventOutcome::Ignored,
        }
    }

    #[cfg(feature = "crossterm")]
    pub fn input_key(&mut self, key: KeyEvent) -> CommandLineEventOutcome {
        if key.kind != KeyEventKind::Press {
            return CommandLineEventOutcome::Ignored;
        }

        if !self.is_active() {
            return match (key.code, key.modifiers) {
                (KeyCode::Char(':'), m) if is_plain_prompt_modifier(m) => {
                    self.open(CommandLineMode::Command);
                    CommandLineEventOutcome::Handled
                }
                (KeyCode::Char('/'), m) if is_plain_prompt_modifier(m) => {
                    self.open(CommandLineMode::SearchForward);
                    CommandLineEventOutcome::Handled
                }
                (KeyCode::Char('?'), m) if is_plain_prompt_modifier(m) => {
                    self.open(CommandLineMode::SearchBackward);
                    CommandLineEventOutcome::Handled
                }
                _ => CommandLineEventOutcome::Ignored,
            };
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.cancel(),
            (KeyCode::Enter, _) => self.submit(),
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.history_previous()
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.history_next()
            }
            _ => match self.input.input(key) {
                TextInputEventOutcome::Ignored => CommandLineEventOutcome::Ignored,
                TextInputEventOutcome::Handled | TextInputEventOutcome::Submitted => {
                    CommandLineEventOutcome::Handled
                }
            },
        }
    }
}
