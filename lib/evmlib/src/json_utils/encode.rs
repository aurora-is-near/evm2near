//! A collection of functions for encoding ethabi::Token values into JSON.

use crate::env::ExitStatus;
use abi_types::ethabi;

/// Given a string which lists the ABI types of a function's output, the exit status, and the
/// return data from the EVM execution; this function attempts to create a json object to encode
/// this output and returns it serialized into bytes.
pub fn encode_return_data_as_json(
    output_types: &[u8],
    return_data: &[u8],
    exit_status: &Option<ExitStatus>,
) -> Result<Vec<u8>, EncodeReturnDataError> {
    let exit_status = exit_status.ok_or(EncodeReturnDataError::NoExitStatus)?;
    let mut json_result = serde_json::Map::new();
    match exit_status {
        ExitStatus::Success => {
            json_result.insert("status".into(), serde_json::Value::String("SUCCESS".into()));
            let output_types = std::str::from_utf8(output_types)
                .map_err(|_| EncodeReturnDataError::InvalidUtf8String)?;

            let mut abi_types = Vec::new();
            for output in output_types.split(',') {
                let abi_type = abi_types::parse_param_type(output)
                    .map_err(|_| EncodeReturnDataError::InvalidAbiType)?;
                abi_types.push(abi_type);
            }
            let mut return_tokens = ethabi::decode(&abi_types, return_data)
                .map_err(|_| EncodeReturnDataError::ReturnDataDecodeFailure)?;
            let json_value = if return_tokens.len() == 1 {
                // unwrap is safe because we checked the length
                ethabi_token_to_json_value(return_tokens.pop().unwrap())
            } else {
                ethabi_token_to_json_value(ethabi::Token::Tuple(return_tokens))
            };
            json_result.insert("output".into(), json_value);
        }
        ExitStatus::Revert => {
            json_result.insert("status".into(), serde_json::Value::String("REVERT".into()));
            // Check for standard Solidity error format
            if return_data[0..4] == [0x08, 0xc3, 0x79, 0xa0] {
                let mut return_tokens =
                    ethabi::decode(&[ethabi::ParamType::String], &return_data[4..])
                        .map_err(|_| EncodeReturnDataError::ReturnDataDecodeFailure)?;
                // Unwrap is statically safe because we passed only a single type to decode
                let error_message = return_tokens.pop().unwrap();
                let json_value = ethabi_token_to_json_value(error_message);
                json_result.insert("error".into(), json_value);
            } else {
                let json_value =
                    serde_json::Value::String(format!("0x{}", hex::encode(return_data)));
                json_result.insert("error".into(), json_value);
            }
        }
        ExitStatus::OutOfGas => {
            json_result.insert(
                "status".into(),
                serde_json::Value::String("OUT_OF_GAS".into()),
            );
        }
    }
    let json_data = serde_json::to_vec(&json_result)
        .map_err(|_| EncodeReturnDataError::JsonSerializationFailure)?;
    Ok(json_data)
}

fn ethabi_token_to_json_value(token: ethabi::Token) -> serde_json::Value {
    match token {
        ethabi::Token::Address(address) => {
            serde_json::Value::String(format!("0x{}", hex::encode(address.as_bytes())))
        }
        ethabi::Token::FixedBytes(bytes) => {
            serde_json::Value::String(format!("0x{}", hex::encode(bytes)))
        }
        ethabi::Token::Bytes(bytes) => {
            serde_json::Value::String(format!("0x{}", hex::encode(bytes)))
        }
        ethabi::Token::Int(number) => {
            let be_bytes = {
                let mut buf = [0u8; 32];
                number.to_big_endian(&mut buf);
                buf
            };
            let signed_number = ethnum::i256::from_be_bytes(be_bytes);
            match i64::try_from(signed_number) {
                Ok(n) => serde_json::Value::Number(serde_json::value::Number::from(n)),
                Err(_) => serde_json::Value::String(signed_number.to_string()),
            }
        }
        ethabi::Token::Uint(number) => match u64::try_from(number) {
            Ok(n) => serde_json::Value::Number(serde_json::value::Number::from(n)),
            Err(_) => serde_json::Value::String(number.to_string()),
        },
        ethabi::Token::Bool(value) => serde_json::Value::Bool(value),
        ethabi::Token::String(value) => serde_json::Value::String(value),
        ethabi::Token::FixedArray(values) => {
            let inner_values = values.into_iter().map(ethabi_token_to_json_value).collect();
            serde_json::Value::Array(inner_values)
        }
        ethabi::Token::Array(values) => {
            let inner_values = values.into_iter().map(ethabi_token_to_json_value).collect();
            serde_json::Value::Array(inner_values)
        }
        ethabi::Token::Tuple(values) => {
            let inner_values = values.into_iter().map(ethabi_token_to_json_value).collect();
            serde_json::Value::Array(inner_values)
        }
    }
}

#[derive(Debug)]
pub enum EncodeReturnDataError {
    NoExitStatus,
    InvalidUtf8String,
    InvalidAbiType,
    ReturnDataDecodeFailure,
    JsonSerializationFailure,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_encode_return_data_as_json() {
        let output = super::encode_return_data_as_json(
            b"int256",
            &hex::decode("000000000000000000000000000000000000000000000000000000000000002A")
                .unwrap(),
            &Some(crate::env::ExitStatus::Success),
        )
        .unwrap();
        let expected_output = r#"{"output":42,"status":"SUCCESS"}"#.as_bytes();
        assert_eq!(&output, expected_output);
    }
}
