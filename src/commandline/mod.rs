//! Vim/Helix-style command line convenience exports.

pub mod input;
pub mod registry;
pub mod state;

#[cfg(feature = "gui")]
pub mod widget;

pub use registry::{
    CommandLineCommand, CommandLineCommandInvocation, CommandLineDispatchError,
    CommandLineParseError, CommandLineParsedCommand, CommandLineRegistrationError,
    CommandLineRegistry, parse_command_args, parse_command_line,
};
pub use state::{CommandLineEventOutcome, CommandLineMode, CommandLineState, CommandLineSubmit};

#[cfg(feature = "gui")]
pub use widget::{CommandLine, CommandLinePlacement};
