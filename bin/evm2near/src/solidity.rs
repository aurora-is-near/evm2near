// This is free and unencumbered software released into the public domain.

use evm_rs::Program;
use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

use crate::{decode::decode_bytecode, error::CompileError};

pub const SOLC: &str = "solc";

pub fn command() -> Command {
    Command::new(SOLC)
}

#[allow(dead_code)]
pub fn is_available() -> bool {
    let subprocess = command()
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match subprocess {
        Err(_) => false,
        Ok(mut child) => match child.wait() {
            Err(_) => false,
            Ok(_) => true,
        },
    }
}

pub fn execute(input_path: &Path, _: Option<&Path>) -> Result<Output, CompileError> {
    let subprocess = command()
        .args(["--bin", "--metadata-hash", "none"])
        .arg(input_path.as_os_str())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    match subprocess {
        Err(_err) => Err(CompileError::ProgramSpawn),
        Ok(child) => match child.wait_with_output() {
            Err(_) => Err(CompileError::ProgramWait),
            Ok(output) => Ok(output),
        },
    }
}

pub fn compile(input_path: &Path) -> Result<Program, CompileError> {
    let output = execute(input_path, None)?;
    match output.status.code() {
        Some(0) => {
            let output = String::from_utf8_lossy(&output.stdout);
            match output.find("Binary:\n") {
                None => Err(CompileError::UnexpectedOutput),
                Some(pos) => match decode_bytecode(&output[pos + 8..]) {
                    Err(err) => Err(CompileError::Decode(err)),
                    Ok(program) => Ok(program),
                },
            }
        }
        Some(code) => Err(CompileError::UnexpectedExit(code, output.stderr)),
        None => Err(CompileError::UnexpectedSignal(output.stderr)),
    }
}
