// This is free and unencumbered software released into the public domain.

use serde::Deserialize;

/// See: https://docs.soliditylang.org/en/v0.8.16/types.html
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum ValueType {
    Address,
    AddressPayable,
    Bytes,
    Bytes32,
    Bool,
    Function,
    Int8,
    Int256,
    String,
    Uint8,
    Uint256,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum StateMutability {
    Nonpayable,
    Payable,
    Pure,
    View,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Parameter {
    name: String,
    r#type: ValueType,
    internal_type: ValueType,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Function {
    name: String,
    inputs: Vec<Parameter>,
    outputs: Vec<Parameter>,
    state_mutability: StateMutability,
    r#type: String,
}

#[allow(dead_code)]
pub fn parse(json: &str) -> Result<Vec<Function>, serde_json::Error> {
    serde_json::from_str::<Vec<Function>>(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    static MULTIPLY: &str = r#"[
        {
            "name":"multiply",
            "type":"function",
            "inputs":[
                {"internalType":"int256","name":"a","type":"int256"},
                {"internalType":"int256","name":"b","type":"int256"}
            ],
            "outputs":[
                {"internalType":"int256","name":"","type":"int256"}
            ],
            "stateMutability":"pure"
        }
    ]"#;

    #[test]
    fn test_parse() {
        let parsed = vec![Function {
            name: "multiply".to_string(),
            inputs: vec![
                Parameter {
                    name: "a".to_string(),
                    r#type: ValueType::Int256,
                    internal_type: ValueType::Int256,
                },
                Parameter {
                    name: "b".to_string(),
                    r#type: ValueType::Int256,
                    internal_type: ValueType::Int256,
                },
            ],
            outputs: vec![Parameter {
                name: "".to_string(),
                r#type: ValueType::Int256,
                internal_type: ValueType::Int256,
            }],
            state_mutability: StateMutability::Pure,
            r#type: "function".to_string(),
        }];
        assert_eq!(parse(MULTIPLY).unwrap(), parsed);
    }
}
