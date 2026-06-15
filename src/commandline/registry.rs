use std::collections::HashMap;

/// Parsed command-line input split into a command name and raw arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLineParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

/// A command that an application can register for command-line dispatch.
///
/// The registry resolves command names, aliases, and optional multi-token
/// patterns. It does not execute commands; host applications own behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLineCommand {
    pub name: String,
    pub aliases: Vec<String>,
    pub patterns: Vec<Vec<String>>,
    pub description: Option<String>,
}

impl CommandLineCommand {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            patterns: Vec::new(),
            description: None,
        }
    }

    pub fn alias<S: Into<String>>(mut self, alias: S) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub fn aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases.extend(aliases.into_iter().map(Into::into));
        self
    }

    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn pattern<I, S>(mut self, pattern: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.patterns
            .push(pattern.into_iter().map(Into::into).collect());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineRegistrationError {
    EmptyName,
    EmptyAlias,
    NameOrAliasContainsWhitespace { value: String },
    DuplicateNameOrAlias { value: String },
    EmptyPattern,
    EmptyPatternPart { pattern: Vec<String> },
    DuplicatePattern { pattern: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineParseError {
    Empty,
    UnterminatedQuote { quote: char },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineDispatchError {
    Parse(CommandLineParseError),
    UnknownCommand { name: String },
}

/// A registered command resolved from user-entered command-line text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLineCommandInvocation {
    pub command: CommandLineCommand,
    pub parsed: CommandLineParsedCommand,
    pub remaining_args: Vec<String>,
}

/// Registry for app-owned command-line commands.
///
/// `dispatch` parses user input and resolves it to a registered command by
/// trying the longest registered pattern first, then falling back to the first
/// token as a command name or alias.
#[derive(Debug, Default, Clone)]
pub struct CommandLineRegistry {
    commands: Vec<CommandLineCommand>,
    aliases: HashMap<String, usize>,
    patterns: Vec<Vec<String>>,
}

impl CommandLineRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        command: CommandLineCommand,
    ) -> Result<&mut Self, CommandLineRegistrationError> {
        self.validate_command(&command)?;

        let idx = self.commands.len();
        self.aliases.insert(command.name.clone(), idx);
        for alias in &command.aliases {
            self.aliases.insert(alias.clone(), idx);
        }
        for pattern in &command.patterns {
            self.patterns.push(pattern.clone());
        }
        self.commands.push(command);
        Ok(self)
    }

    pub fn command(&self, name_or_alias: &str) -> Option<&CommandLineCommand> {
        self.aliases
            .get(name_or_alias)
            .and_then(|idx| self.commands.get(*idx))
    }

    pub fn commands(&self) -> &[CommandLineCommand] {
        &self.commands
    }

    pub fn dispatch(
        &self,
        input: &str,
    ) -> Result<CommandLineCommandInvocation, CommandLineDispatchError> {
        let args = parse_command_args(input).map_err(CommandLineDispatchError::Parse)?;
        if args.is_empty() {
            return Err(CommandLineDispatchError::Parse(
                CommandLineParseError::Empty,
            ));
        }

        if let Some((matched_len, command)) = self.longest_pattern_match(&args) {
            return Ok(CommandLineCommandInvocation {
                command: command.clone(),
                parsed: CommandLineParsedCommand {
                    name: args[0].clone(),
                    args: args[1..].to_vec(),
                },
                remaining_args: args[matched_len..].to_vec(),
            });
        }

        let parsed = CommandLineParsedCommand {
            name: args[0].clone(),
            args: args[1..].to_vec(),
        };
        let Some(command) = self.command(&parsed.name) else {
            return Err(CommandLineDispatchError::UnknownCommand { name: parsed.name });
        };

        Ok(CommandLineCommandInvocation {
            command: command.clone(),
            remaining_args: parsed.args.clone(),
            parsed,
        })
    }

    fn longest_pattern_match(&self, args: &[String]) -> Option<(usize, &CommandLineCommand)> {
        let mut best: Option<(usize, &CommandLineCommand)> = None;
        for command in &self.commands {
            for pattern in &command.patterns {
                if pattern.len() <= args.len()
                    && pattern
                        .iter()
                        .zip(args.iter())
                        .all(|(expected, actual)| expected == actual)
                {
                    if best
                        .map(|(best_len, _)| pattern.len() > best_len)
                        .unwrap_or(true)
                    {
                        best = Some((pattern.len(), command));
                    }
                }
            }
        }
        best
    }

    fn validate_command(
        &self,
        command: &CommandLineCommand,
    ) -> Result<(), CommandLineRegistrationError> {
        validate_name_or_alias(&command.name, true)?;
        if self.aliases.contains_key(&command.name) {
            return Err(CommandLineRegistrationError::DuplicateNameOrAlias {
                value: command.name.clone(),
            });
        }

        let mut local_aliases = vec![command.name.clone()];
        for alias in &command.aliases {
            validate_name_or_alias(alias, false)?;
            if self.aliases.contains_key(alias)
                || local_aliases.iter().any(|existing| existing == alias)
            {
                return Err(CommandLineRegistrationError::DuplicateNameOrAlias {
                    value: alias.clone(),
                });
            }
            local_aliases.push(alias.clone());
        }

        let mut local_patterns: Vec<Vec<String>> = Vec::new();
        for pattern in &command.patterns {
            if pattern.is_empty() {
                return Err(CommandLineRegistrationError::EmptyPattern);
            }
            if pattern.iter().any(|part| part.is_empty()) {
                return Err(CommandLineRegistrationError::EmptyPatternPart {
                    pattern: pattern.clone(),
                });
            }
            if self.patterns.iter().any(|existing| existing == pattern)
                || local_patterns.iter().any(|existing| existing == pattern)
            {
                return Err(CommandLineRegistrationError::DuplicatePattern {
                    pattern: pattern.clone(),
                });
            }
            local_patterns.push(pattern.clone());
        }

        Ok(())
    }
}

fn validate_name_or_alias(value: &str, is_name: bool) -> Result<(), CommandLineRegistrationError> {
    if value.is_empty() {
        return if is_name {
            Err(CommandLineRegistrationError::EmptyName)
        } else {
            Err(CommandLineRegistrationError::EmptyAlias)
        };
    }

    if value.chars().any(char::is_whitespace) {
        return Err(
            CommandLineRegistrationError::NameOrAliasContainsWhitespace {
                value: value.to_string(),
            },
        );
    }

    Ok(())
}

pub fn parse_command_line(input: &str) -> Result<CommandLineParsedCommand, CommandLineParseError> {
    let args = parse_command_args(input)?;
    let Some((name, args)) = args.split_first() else {
        return Err(CommandLineParseError::Empty);
    };

    Ok(CommandLineParsedCommand {
        name: name.clone(),
        args: args.to_vec(),
    })
}

pub fn parse_command_args(input: &str) -> Result<Vec<String>, CommandLineParseError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote: Option<char> = None;
    let mut arg_started = false;

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => {
                quote = None;
                arg_started = true;
            }
            Some(_) if ch == '\\' => {
                arg_started = true;
                if let Some(next) = chars.next() {
                    current.push(next);
                } else {
                    current.push(ch);
                }
            }
            Some(_) => {
                arg_started = true;
                current.push(ch);
            }
            None if ch == '\'' || ch == '"' || ch == '`' => {
                quote = Some(ch);
                arg_started = true;
            }
            None if ch == '\\' => {
                arg_started = true;
                if let Some(next) = chars.next() {
                    current.push(next);
                } else {
                    current.push(ch);
                }
            }
            None if ch.is_whitespace() => {
                if arg_started {
                    args.push(std::mem::take(&mut current));
                    arg_started = false;
                }
            }
            None => {
                arg_started = true;
                current.push(ch);
            }
        }
    }

    if let Some(quote) = quote {
        return Err(CommandLineParseError::UnterminatedQuote { quote });
    }

    if arg_started {
        args.push(current);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::{
        CommandLineCommand, CommandLineDispatchError, CommandLineParseError,
        CommandLineRegistrationError, CommandLineRegistry, parse_command_args, parse_command_line,
    };

    #[test]
    fn parses_command_name_and_arguments() {
        let parsed = parse_command_line("open README.md 'file with spaces.txt'").unwrap();
        assert_eq!(parsed.name, "open");
        assert_eq!(
            parsed.args,
            vec!["README.md".to_string(), "file with spaces.txt".to_string()]
        );
    }

    #[test]
    fn parses_empty_quoted_arguments() {
        assert_eq!(
            parse_command_args("set-option title \"\"").unwrap(),
            vec!["set-option".to_string(), "title".to_string(), String::new()]
        );
    }

    #[test]
    fn parses_backslash_escaped_spaces() {
        assert_eq!(
            parse_command_args("open a\\ b.txt").unwrap(),
            vec!["open".to_string(), "a b.txt".to_string()]
        );
    }

    #[test]
    fn parses_backslash_escapes_inside_quotes() {
        assert_eq!(
            parse_command_args("open \"a \\\"quoted\\\" file\"").unwrap(),
            vec!["open".to_string(), "a \"quoted\" file".to_string()]
        );
    }

    #[test]
    fn reports_unterminated_quotes() {
        assert_eq!(
            parse_command_args("open 'missing").unwrap_err(),
            CommandLineParseError::UnterminatedQuote { quote: '\'' }
        );
    }

    #[test]
    fn registry_dispatch_resolves_aliases() {
        let mut registry = CommandLineRegistry::new();
        registry
            .register(CommandLineCommand::new("set-number").alias("nu"))
            .unwrap();

        let invocation = registry.dispatch("nu relative").unwrap();

        assert_eq!(invocation.command.name, "set-number");
        assert_eq!(invocation.parsed.name, "nu");
        assert_eq!(invocation.parsed.args, vec!["relative".to_string()]);
        assert_eq!(invocation.remaining_args, vec!["relative".to_string()]);
    }

    #[test]
    fn registry_dispatch_resolves_multi_word_commands() {
        let mut registry = CommandLineRegistry::new();
        registry
            .register(CommandLineCommand::new("set-number").pattern(["set", "number"]))
            .unwrap();

        let invocation = registry.dispatch("set number now").unwrap();

        assert_eq!(invocation.command.name, "set-number");
        assert_eq!(invocation.parsed.name, "set");
        assert_eq!(
            invocation.parsed.args,
            vec!["number".to_string(), "now".to_string()]
        );
        assert_eq!(invocation.remaining_args, vec!["now".to_string()]);
    }

    #[test]
    fn registry_dispatch_uses_longest_pattern() {
        let mut registry = CommandLineRegistry::new();
        registry
            .register(CommandLineCommand::new("set").pattern(["set"]))
            .unwrap();
        registry
            .register(CommandLineCommand::new("set-number").pattern(["set", "number"]))
            .unwrap();

        let invocation = registry.dispatch("set number").unwrap();

        assert_eq!(invocation.command.name, "set-number");
        assert!(invocation.remaining_args.is_empty());
    }

    #[test]
    fn registry_dispatch_reports_unknown_command() {
        let registry = CommandLineRegistry::new();

        assert_eq!(
            registry.dispatch("missing").unwrap_err(),
            CommandLineDispatchError::UnknownCommand {
                name: "missing".to_string()
            }
        );
    }

    #[test]
    fn registry_rejects_duplicate_aliases() {
        let mut registry = CommandLineRegistry::new();
        registry
            .register(CommandLineCommand::new("write").alias("w"))
            .unwrap();

        assert_eq!(
            registry
                .register(CommandLineCommand::new("other").alias("w"))
                .unwrap_err(),
            CommandLineRegistrationError::DuplicateNameOrAlias {
                value: "w".to_string()
            }
        );
    }

    #[test]
    fn registry_rejects_duplicate_patterns() {
        let mut registry = CommandLineRegistry::new();
        registry
            .register(CommandLineCommand::new("set-number").pattern(["set", "number"]))
            .unwrap();

        assert_eq!(
            registry
                .register(CommandLineCommand::new("other").pattern(["set", "number"]))
                .unwrap_err(),
            CommandLineRegistrationError::DuplicatePattern {
                pattern: vec!["set".to_string(), "number".to_string()]
            }
        );
    }
}
