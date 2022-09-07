// This is free and unencumbered software released into the public domain.

use evm_rs::{decode_program, DecodeError, Program};

pub fn decode_bytecode(input: &str) -> Result<Program, DecodeError> {
    let input = input.trim();
    let input = if input.starts_with("0x") || input.starts_with("0X") {
        &input[2..]
    } else {
        input
    };
    let input = match input.find("a164736f6c63") {
        Some(n) => &input[..n],
        None => input,
    };
    match hex::decode(input) {
        Err(_err) => Err(DecodeError::InvalidBytecode),
        Ok(bytecode) => match decode_program(bytecode.as_slice()) {
            Err(err) => Err(err),
            Ok(program) => Ok(program),
        },
    }
}
