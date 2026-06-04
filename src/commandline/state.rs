use crate::TextInputState;

/// Active prompt mode for the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLineMode {
    /// `:` command prompt.
    Command,
    /// `/` forward search prompt.
    SearchForward,
    /// `?` backward search prompt.
    SearchBackward,
}

impl CommandLineMode {
    /// Prompt text shown for this mode.
    pub fn prompt(self) -> &'static str {
        match self {
            Self::Command => ":",
            Self::SearchForward => "/",
            Self::SearchBackward => "?",
        }
    }
}

/// Submitted command-line input.
///
/// The command line captures input and reports what was submitted. The host
/// application decides how to execute commands and search requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineSubmit {
    Command(String),
    SearchForward(String),
    SearchBackward(String),
}

impl CommandLineSubmit {
    /// Mode used when the input was submitted.
    pub fn mode(&self) -> CommandLineMode {
        match self {
            Self::Command(_) => CommandLineMode::Command,
            Self::SearchForward(_) => CommandLineMode::SearchForward,
            Self::SearchBackward(_) => CommandLineMode::SearchBackward,
        }
    }

    /// Submitted input without the prompt prefix.
    pub fn input(&self) -> &str {
        match self {
            Self::Command(input) => input,
            Self::SearchForward(input) => input,
            Self::SearchBackward(input) => input,
        }
    }
}

/// Result of feeding input to a [`CommandLineState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineEventOutcome {
    /// The command line did not consume the event.
    Ignored,
    /// The command line consumed the event without submitting or cancelling.
    Handled,
    /// The active prompt was cancelled.
    Cancelled,
    /// Input was submitted.
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
    /// Create an inactive command-line state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the command line is currently accepting input.
    pub fn is_active(&self) -> bool {
        self.mode.is_some()
    }

    /// Active command-line mode.
    pub fn mode(&self) -> Option<CommandLineMode> {
        self.mode
    }

    /// Active prompt text, or an empty string when inactive.
    pub fn prompt(&self) -> &'static str {
        self.mode.map(CommandLineMode::prompt).unwrap_or("")
    }

    /// Current input without the prompt prefix.
    pub fn input(&self) -> &str {
        self.input.current_text()
    }

    /// Replace the current input and reset history navigation.
    pub fn set_input<S: Into<String>>(&mut self, input: S) {
        self.input.set_text(input);
        self.history_cursor = None;
    }

    /// Access the embedded single-line text input state.
    pub fn text_input(&self) -> &TextInputState {
        &self.input
    }

    /// Mutably access the embedded single-line text input state.
    pub fn text_input_mut(&mut self) -> &mut TextInputState {
        &mut self.input
    }

    /// Maximum number of submitted entries retained in history.
    pub fn history_limit(&self) -> usize {
        self.history_limit
    }

    /// Set the maximum number of submitted entries retained in history.
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit;
        if self.history.len() > limit {
            let extra = self.history.len() - limit;
            self.history.drain(0..extra);
        }
        self.history_cursor = None;
    }

    /// Open the command line in the requested mode with empty input.
    pub fn open(&mut self, mode: CommandLineMode) {
        self.mode = Some(mode);
        self.input.set_text("");
        self.input.enter_edit_mode();
        self.history_cursor = None;
    }

    /// Open the command line in the requested mode with prefilled input.
    pub fn open_with_input<S: Into<String>>(&mut self, mode: CommandLineMode, input: S) {
        self.mode = Some(mode);
        self.input.set_text(input);
        self.input.enter_edit_mode();
        self.history_cursor = None;
    }

    /// Open `:` command mode.
    pub fn open_command(&mut self) {
        self.open(CommandLineMode::Command);
    }

    /// Open `/` forward search mode.
    pub fn open_search_forward(&mut self) {
        self.open(CommandLineMode::SearchForward);
    }

    /// Open `?` backward search mode.
    pub fn open_search_backward(&mut self) {
        self.open(CommandLineMode::SearchBackward);
    }

    /// Cancel the active command line.
    pub fn cancel(&mut self) -> CommandLineEventOutcome {
        if !self.is_active() {
            return CommandLineEventOutcome::Ignored;
        }
        self.close();
        CommandLineEventOutcome::Cancelled
    }

    /// Close the command line and clear current input.
    pub fn close(&mut self) {
        self.mode = None;
        self.input.set_text("");
        self.history_cursor = None;
    }

    /// Submit the active command-line input.
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

    /// Move to the previous history entry.
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

    /// Move to the next history entry.
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
    /// Cursor position for the default bottom-row command-line placement.
    pub fn cursor(&self, area: ratatui::layout::Rect) -> (u16, u16) {
        let area = crate::commandline::CommandLine::placement_area(
            area,
            crate::commandline::CommandLinePlacement::Bottom,
        );
        self.cursor_inline(area)
    }

    #[cfg(feature = "gui")]
    /// Cursor position when the command line is rendered into the given area exactly.
    pub fn cursor_inline(&self, area: ratatui::layout::Rect) -> (u16, u16) {
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
