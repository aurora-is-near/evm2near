// This is free and unencumbered software released into the public domain.

#[allow(unused_imports)]
use crate::{
    env::Env,
    ops::{ENV, EVM},
    state::{Word, ZERO},
};
use abi_types::ethabi::{
    self,
    ethereum_types::{H160, U256},
};

#[no_mangle]
pub static mut _abi_buffer: [u8; 0xFFFF] = [1; 0xFFFF]; // FIXME

#[no_mangle]
pub unsafe fn _evm_init(_table_offset: u32, chain_id: u64, balance: u64) {
    #[cfg(feature = "near")]
    {
        // TODO
    }

    #[cfg(not(feature = "near"))]
    {
        let mut args = std::env::args();

        // Remove fluff from the command-line arguments:
        let mut arg = args.next();
        let mut arg_pos = 0;
        loop {
            match &arg {
                None => break, // no more arguments
                Some(s) => {
                    if arg_pos == 0 && (s.ends_with(".wasm") || s.ends_with(".wasi")) {
                        // consume the program name
                    } else {
                        match s.as_str() {
                            "--" => {
                                arg = args.next(); // start of actual arguments
                                break;
                            }
                            "--func" | "--invoke" => _ = args.next(), // skip interpreter options
                            _ => break,                               // start of actual arguments
                        }
                    }
                }
            }
            arg = args.next();
            arg_pos += 1;
        }

        ENV.call_data = match arg {
            None => Vec::new(), // no call data given
            Some(input) => {
                if input.starts_with("0x") {
                    match hex::decode(&input[2..]) {
                        Err(err) => panic!("{}", err), // FIXME
                        Ok(bytes) => bytes,
                    }
                } else if input.starts_with("{") || input.starts_with("[") {
                    input.into_bytes() // JSON
                } else {
                    panic!("expected JSON or hexadecimal input, but got: {}", input);
                    // FIXME
                }
            }
        };

        EVM.trace_level = 0; // TODO: look for --trace in args

        EVM.call_value = match args.next() {
            None => ZERO,
            Some(s) => Word::from(s.parse::<u64>().unwrap_or(0)), // TODO: support decimal point as well
        };
        //eprintln!("_evm_init: call_data={:?} call_value={:?}", ENV.call_data, EVM.call_value);
    }

    EVM.chain_id = Word::from(chain_id);
    EVM.self_balance = Word::from(balance);
}

#[no_mangle]
pub unsafe fn _evm_call(
    selector: u32,
    param_names_off: usize, // relative to _abi_buffer
    param_names_len: usize,
    param_types_off: usize, // relative to _abi_buffer
    param_types_len: usize,
) {
    let raw_call_data = ENV.call_data();

    let param_names_ptr: *mut u8 = _abi_buffer
        .as_mut_ptr()
        .offset(param_names_off.try_into().unwrap());
    let param_names = std::slice::from_raw_parts(param_names_ptr, param_names_len);

    let param_types_ptr: *mut u8 = _abi_buffer
        .as_mut_ptr()
        .offset(param_types_off.try_into().unwrap());
    let param_types = std::slice::from_raw_parts(param_types_ptr, param_types_len);

    let call_data = if param_names.is_empty() {
        let mut call_data: Vec<u8> = vec![0; 4 + raw_call_data.len()];
        call_data[0..4].copy_from_slice(&selector.to_be_bytes());
        call_data[4..].copy_from_slice(raw_call_data);
        call_data
    } else {
        // TODO: support raw call data as well
        // TODO: check that sufficient arguments were provided
        transform_json_call_data(selector, param_names, param_types, raw_call_data).unwrap()
    };

    #[cfg(all(feature = "near", not(test)))]
    {
        ENV.call_data = Some(call_data);
    }
    #[cfg(any(not(feature = "near"), test))]
    {
        ENV.call_data = call_data;
    }
}

#[no_mangle]
pub unsafe fn _evm_pop_u32() -> u32 {
    EVM.stack.pop().as_u32()
}

#[no_mangle]
pub unsafe fn _evm_set_pc(pc: u32) {
    #[cfg(feature = "pc")]
    EVM.program_counter = pc;
}

/// Transforms the given call_data (assumed to be json format) into solidity-encoded input
/// using the given ABI (parameter names and types).
fn transform_json_call_data(
    selector: u32,
    param_names: &[u8],
    param_types: &[u8],
    json_call_data: &[u8],
) -> Result<Vec<u8>, TransformCallDataError> {
    let param_names =
        std::str::from_utf8(param_names).map_err(|_| TransformCallDataError::InvalidUtf8String)?;
    let param_types =
        std::str::from_utf8(param_types).map_err(|_| TransformCallDataError::InvalidUtf8String)?;
    assert_eq!(
        param_names.split(',').count(),
        param_types.split(',').count(),
        "Expected same number of parameter names and types"
    );
    let parsed_json: serde_json::Value =
        serde_json::from_slice(json_call_data).map_err(|_| TransformCallDataError::InvalidJson)?;
    let json_object = parsed_json
        .as_object()
        .ok_or(TransformCallDataError::NotJsonObject)?;
    let mut abi_tokens: Vec<ethabi::Token> = Vec::with_capacity(param_names.len());
    for (name, typ) in param_names.split(',').zip(param_types.split(',')) {
        let abi_type =
            abi_types::parse_param_type(typ).map_err(|_| TransformCallDataError::InvalidAbiType)?;
        let param_value = json_object
            .get(name)
            .ok_or(TransformCallDataError::MissingParameter)?;
        let abi_value = parse_json_value_to_abi_type(param_value, &abi_type)?;
        abi_tokens.push(abi_value);
    }
    let args = ethabi::encode(&abi_tokens);
    let selector_bytes: &[u8] = &selector.to_be_bytes();
    Ok([selector_bytes, &args].concat())
}

fn parse_json_value_to_abi_type(
    param_value: &serde_json::Value,
    abi_type: &ethabi::ParamType,
) -> Result<ethabi::Token, TransformCallDataError> {
    match abi_type {
        ethabi::ParamType::Address => {
            let address_bytes = param_value
                .as_str()
                .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            let address_bytes: [u8; 20] = address_bytes
                .try_into()
                .map_err(|_| TransformCallDataError::InvalidAbiValue)?;
            Ok(ethabi::Token::Address(H160(address_bytes)))
        }
        ethabi::ParamType::Bytes => {
            let bytes = param_value
                .as_str()
                .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            Ok(ethabi::Token::Bytes(bytes))
        }
        ethabi::ParamType::Int(_) => {
            let number = parse_json_value_to_i256(param_value)?;
            Ok(ethabi::Token::Int(number))
        }
        ethabi::ParamType::Uint(_) => {
            let number = parse_json_value_to_u256(param_value)?;
            Ok(ethabi::Token::Uint(number))
        }
        ethabi::ParamType::Bool => param_value
            .as_bool()
            .map(ethabi::Token::Bool)
            .ok_or(TransformCallDataError::InvalidAbiValue),
        ethabi::ParamType::String => param_value
            .as_str()
            .map(|s| ethabi::Token::String(String::from(s)))
            .ok_or(TransformCallDataError::InvalidAbiValue),
        ethabi::ParamType::Array(inner_abi_type) => {
            let inner_values = param_value
                .as_array()
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            let mut inner_abi_values = Vec::with_capacity(inner_values.len());
            for v in inner_values {
                inner_abi_values.push(parse_json_value_to_abi_type(v, inner_abi_type)?);
            }
            Ok(ethabi::Token::Array(inner_abi_values))
        }
        ethabi::ParamType::FixedBytes(fixed_len) => {
            let bytes = param_value
                .as_str()
                .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            if &bytes.len() != fixed_len {
                return Err(TransformCallDataError::InvalidAbiValue);
            }
            Ok(ethabi::Token::FixedBytes(bytes))
        }
        ethabi::ParamType::FixedArray(inner_abi_type, fixed_len) => {
            let inner_values = param_value
                .as_array()
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            if &inner_values.len() != fixed_len {
                return Err(TransformCallDataError::InvalidAbiValue);
            }
            let mut inner_abi_values = Vec::with_capacity(inner_values.len());
            for v in inner_values {
                inner_abi_values.push(parse_json_value_to_abi_type(v, inner_abi_type)?);
            }
            Ok(ethabi::Token::FixedArray(inner_abi_values))
        }
        ethabi::ParamType::Tuple(inner_abi_types) => {
            let inner_values = param_value
                .as_array()
                .ok_or(TransformCallDataError::InvalidAbiValue)?;
            if inner_values.len() != inner_abi_types.len() {
                return Err(TransformCallDataError::InvalidAbiValue);
            }
            let mut inner_abi_values = Vec::with_capacity(inner_values.len());
            for (v, t) in inner_values.iter().zip(inner_abi_types.iter()) {
                inner_abi_values.push(parse_json_value_to_abi_type(v, t)?);
            }
            Ok(ethabi::Token::Tuple(inner_abi_values))
        }
    }
}

fn parse_json_value_to_u256(value: &serde_json::Value) -> Result<U256, TransformCallDataError> {
    match value {
        serde_json::Value::String(s) => {
            let parsed = if s.starts_with("0x") {
                ethabi::ethereum_types::U256::from_str_radix(s, 16)
            } else {
                ethabi::ethereum_types::U256::from_str_radix(s, 10)
            };
            parsed.map_err(|_| TransformCallDataError::InvalidAbiValue)
        }
        serde_json::Value::Number(num) => num
            .as_u64()
            .map(U256::from)
            .ok_or(TransformCallDataError::InvalidAbiValue),
        _ => Err(TransformCallDataError::InvalidAbiValue),
    }
}

fn parse_json_value_to_i256(value: &serde_json::Value) -> Result<U256, TransformCallDataError> {
    match value {
        serde_json::Value::String(s) => {
            let parsed = if s.starts_with("0x") {
                ethabi::ethereum_types::U256::from_str_radix(s, 16)
                    .map_err(|_| TransformCallDataError::InvalidAbiValue)?
            } else {
                let number = ethnum::i256::from_str_radix(s, 10)
                    .map_err(|_| TransformCallDataError::InvalidAbiValue)?;
                let bytes = number.to_be_bytes();
                U256::from_big_endian(&bytes)
            };
            Ok(parsed)
        }
        serde_json::Value::Number(num) => num
            .as_i64()
            .map(|i| {
                let number = ethnum::i256::from(i);
                let bytes = number.to_be_bytes();
                U256::from_big_endian(&bytes)
            })
            .ok_or(TransformCallDataError::InvalidAbiValue),
        _ => Err(TransformCallDataError::InvalidAbiValue),
    }
}

#[derive(Debug)]
enum TransformCallDataError {
    InvalidUtf8String,
    InvalidJson,
    NotJsonObject,
    InvalidAbiType,
    MissingParameter,
    InvalidAbiValue,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_transform_json_call_data() {
        let output = super::transform_json_call_data(
            0x3c4308a8,
            b"a,b",
            b"int256,int256",
            r#"{"a": 6, "b": 7}"#.as_bytes(),
        )
        .unwrap();
        let expected_output = hex::decode("3c4308a800000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000007").unwrap();
        assert_eq!(output, expected_output);
    }
}
