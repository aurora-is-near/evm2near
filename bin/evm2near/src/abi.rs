// This is free and unencumbered software released into the public domain.

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Functions(Vec<Function>);

impl Default for Functions {
    fn default() -> Self {
        Self(vec![])
    }
}

impl IntoIterator for Functions {
    type Item = Function;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

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
pub fn parse_str(json: &str) -> Result<Functions, serde_json::Error> {
    serde_json::from_str::<Vec<Function>>(json).map(|fs| Functions(fs))
}

#[allow(dead_code)]
pub fn parse_bytes(json: &[u8]) -> Result<Functions, serde_json::Error> {
    serde_json::from_slice::<Vec<Function>>(json).map(|fs| Functions(fs))
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
        assert_eq!(parse_str(MULTIPLY).unwrap().0, parsed);
    }
}
