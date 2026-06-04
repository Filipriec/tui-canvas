use crate::TextInputState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLineMode {
    Command,
    SearchForward,
    SearchBackward,
}

impl CommandLineMode {
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Command => ":",
            Self::SearchForward => "/",
            Self::SearchBackward => "?",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineSubmit {
    Command(String),
    SearchForward(String),
    SearchBackward(String),
}

impl CommandLineSubmit {
    pub fn mode(&self) -> CommandLineMode {
        match self {
            Self::Command(_) => CommandLineMode::Command,
            Self::SearchForward(_) => CommandLineMode::SearchForward,
            Self::SearchBackward(_) => CommandLineMode::SearchBackward,
        }
    }

    pub fn input(&self) -> &str {
        match self {
            Self::Command(input) => input,
            Self::SearchForward(input) => input,
            Self::SearchBackward(input) => input,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineEventOutcome {
    Ignored,
    Handled,
    Cancelled,
    Submitted(CommandLineSubmit),
}

#[derive(Debug, Clone)]
struct CommandLineHistoryEntry {
    mode: CommandLineMode,
    input: String,
}

pub struct CommandLineState {
    pub(crate) input: TextInputState,
    pub(crate) mode: Option<CommandLineMode>,
    history: Vec<CommandLineHistoryEntry>,
    history_cursor: Option<usize>,
    history_limit: usize,
}

impl Default for CommandLineState {
    fn default() -> Self {
        Self {
            input: TextInputState::default(),
            mode: None,
            history: Vec::new(),
            history_cursor: None,
            history_limit: 100,
        }
    }
}

impl CommandLineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.mode.is_some()
    }

    pub fn mode(&self) -> Option<CommandLineMode> {
        self.mode
    }

    pub fn prompt(&self) -> &'static str {
        self.mode.map(CommandLineMode::prompt).unwrap_or("")
    }

    pub fn input(&self) -> &str {
        self.input.current_text()
    }

    pub fn set_input<S: Into<String>>(&mut self, input: S) {
        self.input.set_text(input);
        self.history_cursor = None;
    }

    pub fn text_input(&self) -> &TextInputState {
        &self.input
    }

    pub fn text_input_mut(&mut self) -> &mut TextInputState {
        &mut self.input
    }

    pub fn history_limit(&self) -> usize {
        self.history_limit
    }

    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit;
        if self.history.len() > limit {
            let extra = self.history.len() - limit;
            self.history.drain(0..extra);
        }
        self.history_cursor = None;
    }

    pub fn open(&mut self, mode: CommandLineMode) {
        self.mode = Some(mode);
        self.input.set_text("");
        self.input.enter_edit_mode();
        self.history_cursor = None;
    }

    pub fn open_with_input<S: Into<String>>(&mut self, mode: CommandLineMode, input: S) {
        self.mode = Some(mode);
        self.input.set_text(input);
        self.input.enter_edit_mode();
        self.history_cursor = None;
    }

    pub fn open_command(&mut self) {
        self.open(CommandLineMode::Command);
    }

    pub fn open_search_forward(&mut self) {
        self.open(CommandLineMode::SearchForward);
    }

    pub fn open_search_backward(&mut self) {
        self.open(CommandLineMode::SearchBackward);
    }

    pub fn cancel(&mut self) -> CommandLineEventOutcome {
        if !self.is_active() {
            return CommandLineEventOutcome::Ignored;
        }
        self.close();
        CommandLineEventOutcome::Cancelled
    }

    pub fn close(&mut self) {
        self.mode = None;
        self.input.set_text("");
        self.history_cursor = None;
    }

    pub fn submit(&mut self) -> CommandLineEventOutcome {
        let Some(mode) = self.mode else {
            return CommandLineEventOutcome::Ignored;
        };

        let input = self.input().to_string();
        let submitted = match mode {
            CommandLineMode::Command => CommandLineSubmit::Command(input.clone()),
            CommandLineMode::SearchForward => CommandLineSubmit::SearchForward(input.clone()),
            CommandLineMode::SearchBackward => CommandLineSubmit::SearchBackward(input.clone()),
        };

        if !input.is_empty() {
            self.push_history(mode, input);
        }
        self.close();
        CommandLineEventOutcome::Submitted(submitted)
    }

    pub fn history_previous(&mut self) -> CommandLineEventOutcome {
        if !self.is_active() || self.history.is_empty() {
            return CommandLineEventOutcome::Ignored;
        }

        let next = self
            .history_cursor
            .map(|idx| idx.saturating_sub(1))
            .unwrap_or_else(|| self.history.len().saturating_sub(1));
        self.apply_history_entry(next)
    }

    pub fn history_next(&mut self) -> CommandLineEventOutcome {
        if !self.is_active() || self.history.is_empty() {
            return CommandLineEventOutcome::Ignored;
        }

        let Some(cursor) = self.history_cursor else {
            return CommandLineEventOutcome::Ignored;
        };

        if cursor + 1 >= self.history.len() {
            self.history_cursor = None;
            self.input.set_text("");
            return CommandLineEventOutcome::Handled;
        }

        self.apply_history_entry(cursor + 1)
    }

    fn apply_history_entry(&mut self, idx: usize) -> CommandLineEventOutcome {
        let Some(entry) = self.history.get(idx).cloned() else {
            return CommandLineEventOutcome::Ignored;
        };

        self.mode = Some(entry.mode);
        self.input.set_text(entry.input);
        self.history_cursor = Some(idx);
        CommandLineEventOutcome::Handled
    }

    fn push_history(&mut self, mode: CommandLineMode, input: String) {
        if self
            .history
            .last()
            .map(|entry| entry.mode == mode && entry.input == input)
            .unwrap_or(false)
        {
            return;
        }

        self.history.push(CommandLineHistoryEntry { mode, input });
        if self.history.len() > self.history_limit {
            self.history.remove(0);
        }
    }

    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: ratatui::layout::Rect) -> (u16, u16) {
        let prompt_width = crate::gui_utils::display_width(self.prompt());
        let input_area = ratatui::layout::Rect {
            x: area.x.saturating_add(prompt_width),
            y: area.y,
            width: area.width.saturating_sub(prompt_width),
            height: area.height,
        };
        self.input.cursor(input_area, None)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandLineEventOutcome, CommandLineMode, CommandLineState, CommandLineSubmit,
    };

    #[test]
    fn submit_command_closes_and_returns_payload() {
        let mut commandline = CommandLineState::new();
        commandline.open_command();
        commandline.set_input("set number");

        assert_eq!(
            commandline.submit(),
            CommandLineEventOutcome::Submitted(CommandLineSubmit::Command(
                "set number".to_string()
            ))
        );
        assert!(!commandline.is_active());
        assert_eq!(commandline.input(), "");
    }

    #[test]
    fn submit_search_modes_return_directional_payloads() {
        let mut commandline = CommandLineState::new();
        commandline.open_search_forward();
        commandline.set_input("needle");
        assert_eq!(
            commandline.submit(),
            CommandLineEventOutcome::Submitted(CommandLineSubmit::SearchForward(
                "needle".to_string()
            ))
        );

        commandline.open_search_backward();
        commandline.set_input("needle");
        assert_eq!(
            commandline.submit(),
            CommandLineEventOutcome::Submitted(CommandLineSubmit::SearchBackward(
                "needle".to_string()
            ))
        );
    }

    #[test]
    fn history_previous_and_next_restore_mode_and_input() {
        let mut commandline = CommandLineState::new();
        commandline.open(CommandLineMode::Command);
        commandline.set_input("set number");
        let _ = commandline.submit();
        commandline.open(CommandLineMode::SearchForward);
        commandline.set_input("needle");
        let _ = commandline.submit();

        commandline.open(CommandLineMode::Command);
        assert_eq!(commandline.history_previous(), CommandLineEventOutcome::Handled);
        assert_eq!(commandline.mode(), Some(CommandLineMode::SearchForward));
        assert_eq!(commandline.input(), "needle");

        assert_eq!(commandline.history_previous(), CommandLineEventOutcome::Handled);
        assert_eq!(commandline.mode(), Some(CommandLineMode::Command));
        assert_eq!(commandline.input(), "set number");

        assert_eq!(commandline.history_next(), CommandLineEventOutcome::Handled);
        assert_eq!(commandline.mode(), Some(CommandLineMode::SearchForward));
        assert_eq!(commandline.input(), "needle");
    }

    #[test]
    fn cancel_closes_active_commandline() {
        let mut commandline = CommandLineState::new();
        commandline.open_command();
        commandline.set_input("write");

        assert_eq!(commandline.cancel(), CommandLineEventOutcome::Cancelled);
        assert!(!commandline.is_active());
        assert_eq!(commandline.input(), "");
    }
}
