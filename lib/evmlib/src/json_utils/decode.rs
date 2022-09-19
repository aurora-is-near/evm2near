//! A collection of functions for decoding JSON into ethabi::Token values.

use abi_types::ethabi::{
    self,
    ethereum_types::{H160, U256},
};

/// Transforms the given call_data (assumed to be json format) into solidity-encoded input
/// using the given ABI (parameter names and types).
pub fn transform_json_call_data(
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
pub enum TransformCallDataError {
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
