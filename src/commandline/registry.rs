use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLineParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

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
pub enum CommandLineParseError {
    Empty,
    UnterminatedQuote { quote: char },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineDispatchError {
    Parse(CommandLineParseError),
    UnknownCommand { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLineCommandInvocation {
    pub command: CommandLineCommand,
    pub parsed: CommandLineParsedCommand,
    pub args: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct CommandLineRegistry {
    commands: Vec<CommandLineCommand>,
    aliases: HashMap<String, usize>,
}

impl CommandLineRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: CommandLineCommand) -> &mut Self {
        let idx = self.commands.len();
        self.aliases.insert(command.name.clone(), idx);
        for alias in &command.aliases {
            self.aliases.insert(alias.clone(), idx);
        }
        self.commands.push(command);
        self
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
        let parsed = parse_command_line(input).map_err(CommandLineDispatchError::Parse)?;
        let Some(command) = self.command(&parsed.name) else {
            return Err(CommandLineDispatchError::UnknownCommand { name: parsed.name });
        };

        Ok(CommandLineCommandInvocation {
            command: command.clone(),
            args: parsed.args.clone(),
            parsed,
        })
    }

    pub fn dispatch_pattern(
        &self,
        input: &str,
    ) -> Result<CommandLineCommandInvocation, CommandLineDispatchError> {
        let args = parse_command_args(input).map_err(CommandLineDispatchError::Parse)?;
        if args.is_empty() {
            return Err(CommandLineDispatchError::Parse(CommandLineParseError::Empty));
        }

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

        if let Some((matched_len, command)) = best {
            return Ok(CommandLineCommandInvocation {
                command: command.clone(),
                parsed: CommandLineParsedCommand {
                    name: args[0].clone(),
                    args: args[1..].to_vec(),
                },
                args: args[matched_len..].to_vec(),
            });
        }

        self.dispatch(input)
    }
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

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => {
                quote = None;
            }
            Some(_) => {
                current.push(ch);
            }
            None if ch == '\'' || ch == '"' || ch == '`' => {
                quote = Some(ch);
            }
            None if ch == '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                } else {
                    current.push(ch);
                }
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            None => {
                current.push(ch);
            }
        }
    }

    if let Some(quote) = quote {
        return Err(CommandLineParseError::UnterminatedQuote { quote });
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_command_args, parse_command_line, CommandLineCommand, CommandLineDispatchError,
        CommandLineParseError, CommandLineRegistry,
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
    fn parses_backslash_escaped_spaces() {
        assert_eq!(
            parse_command_args("open a\\ b.txt").unwrap(),
            vec!["open".to_string(), "a b.txt".to_string()]
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
        registry.register(CommandLineCommand::new("set-number").alias("nu"));

        let invocation = registry.dispatch("nu relative").unwrap();

        assert_eq!(invocation.command.name, "set-number");
        assert_eq!(invocation.parsed.name, "nu");
        assert_eq!(invocation.parsed.args, vec!["relative".to_string()]);
        assert_eq!(invocation.args, vec!["relative".to_string()]);
    }

    #[test]
    fn registry_dispatch_pattern_resolves_multi_word_commands() {
        let mut registry = CommandLineRegistry::new();
        registry.register(
            CommandLineCommand::new("set-number").pattern(["set", "number"]),
        );

        let invocation = registry.dispatch_pattern("set number now").unwrap();

        assert_eq!(invocation.command.name, "set-number");
        assert_eq!(invocation.parsed.name, "set");
        assert_eq!(invocation.parsed.args, vec!["number".to_string(), "now".to_string()]);
        assert_eq!(invocation.args, vec!["now".to_string()]);
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
}
