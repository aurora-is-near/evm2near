// This is free and unencumbered software released into the public domain.

use evm_rs::DecodeError;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    ProgramSpawn,
    ProgramWait,
    Decode(DecodeError),
    UnexpectedOutput,
    UnexpectedExit(i32, Vec<u8>),
    UnexpectedSignal(Vec<u8>),
}

#[cfg(feature = "std")]
impl std::error::Error for CompileError {}

impl CompileError {
    pub fn with_program(&self, name: &str) -> String {
        self.to_string().replace("%s", name)
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CompileError::*;
        match self {
            ProgramSpawn => write!(f, "could not execute `%s'"),
            ProgramWait => write!(f, "could not wait for `%s'"),
            Decode(err) => write!(f, "could not parse `%s' output: {}", err),
            UnexpectedOutput => write!(f, "unexpected output from `%s'"),
            UnexpectedExit(code, stderr) => {
                write!(
                    f,
                    "exit code {} from `%s':\n\n{}",
                    code,
                    &String::from_utf8_lossy(stderr).trim_end()
                )
            }
            UnexpectedSignal(stderr) => {
                write!(
                    f,
                    "signal from `%s':\n\n{}",
                    &String::from_utf8_lossy(stderr).trim_end()
                )
            }
        }
    }
}
