//! Vim/Helix-style command line convenience exports.

pub mod input;
pub mod registry;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use registry::{
    parse_command_args, parse_command_line, CommandLineCommand, CommandLineCommandInvocation,
    CommandLineDispatchError, CommandLineParseError, CommandLineParsedCommand,
    CommandLineRegistry,
};
pub use state::{
    CommandLineEventOutcome, CommandLineMode, CommandLineState, CommandLineSubmit,
};

#[cfg(feature = "gui")]
pub use widget::CommandLine;
