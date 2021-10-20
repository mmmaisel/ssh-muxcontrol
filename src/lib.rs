#![forbid(unsafe_code)]

use std::fmt;

mod commands;
mod session;

pub use commands::CommandError;
pub use session::{run, run_stdin, MuxError, ShellResult};

/// Error returned by ssh-muxcontrol library.
#[derive(Debug)]
pub enum SshctlError {
    CommandError(CommandError),
    MuxError(MuxError),
    IoError(std::io::Error),
}

impl From<CommandError> for SshctlError {
    fn from(err: CommandError) -> Self {
        SshctlError::CommandError(err)
    }
}

impl From<MuxError> for SshctlError {
    fn from(err: MuxError) -> Self {
        SshctlError::MuxError(err)
    }
}

impl From<std::io::Error> for SshctlError {
    fn from(err: std::io::Error) -> Self {
        SshctlError::IoError(err)
    }
}

impl fmt::Display for SshctlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::CommandError(e) => write!(f, "CommandError: {}", e),
            Self::MuxError(e) => write!(f, "MuxError: {}", e),
            Self::IoError(e) => write!(f, "IoError: {}", e),
        }
    }
}

#[cfg(test)]
mod tests;
