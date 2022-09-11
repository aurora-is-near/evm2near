// This is free and unencumbered software released into the public domain.

use evm_rs::Program;
use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Output, Stdio},
};

use crate::{abi::Functions, decode::decode_bytecode, error::CompileError};

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

pub fn execute<I, S>(input_path: &Path, args: I) -> Result<Output, CompileError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let subprocess = command()
        .args(args)
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
    let output = execute(
        input_path,
        ["--bin-runtime", "--optimize", "--metadata-hash", "none"],
    )?;
    match output.status.code() {
        Some(0) => {
            let output = String::from_utf8_lossy(&output.stdout);
            //let marker = "Binary:\n";
            let marker = "Binary of the runtime part:\n";
            match output.find(marker) {
                None => Err(CompileError::UnexpectedOutput),
                Some(pos) => match decode_bytecode(&output[pos + marker.len()..]) {
                    Err(err) => Err(CompileError::Decode(err)),
                    Ok(program) => Ok(program),
                },
            }
        }
        Some(code) => Err(CompileError::UnexpectedExit(code, output.stderr)),
        None => Err(CompileError::UnexpectedSignal(output.stderr)),
    }
}

pub fn compile_abi(input_path: &Path) -> Result<Functions, CompileError> {
    let output = execute(input_path, ["--abi"])?;
    match output.status.code() {
        Some(0) => {
            let output = String::from_utf8_lossy(&output.stdout);
            let marker = "Contract JSON ABI\n";
            match output.find(marker) {
                None => Err(CompileError::UnexpectedOutput),
                Some(pos) => crate::abi::parse_str(&output[pos + marker.len()..])
                    .map_err(|_| CompileError::UnexpectedOutput),
            }
        }
        Some(code) => Err(CompileError::UnexpectedExit(code, output.stderr)),
        None => Err(CompileError::UnexpectedSignal(output.stderr)),
    }
}
