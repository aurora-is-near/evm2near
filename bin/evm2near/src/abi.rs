// This is free and unencumbered software released into the public domain.

use serde::Deserialize;
use sha3::{Digest, Keccak256};
use std::fmt;

#[derive(Deserialize, Debug, PartialEq, Eq, Default)]
pub struct Functions(Vec<Function>);

impl IntoIterator for Functions {
    type Item = Function;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum StateMutability {
    Nonpayable,
    Payable,
    Pure,
    View,
}

impl fmt::Display for StateMutability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StateMutability::*;
        match self {
            Nonpayable => write!(f, "nonpayable"),
            Payable => write!(f, "payable"),
            Pure => write!(f, "pure"),
            View => write!(f, "view"),
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Parameter {
    pub name: String,
    /// Note: this type must be an ABI type, as opposed to a Solidity type.
    /// See https://docs.soliditylang.org/en/develop/abi-spec.html#mapping-solidity-to-abi-types
    /// for the distinction.
    pub r#type: String,
    pub internal_type: Option<String>,
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.r#type)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Function {
    pub name: String,
    pub inputs: Vec<Parameter>,
    pub outputs: Vec<Parameter>,
    pub state_mutability: StateMutability,
    pub r#type: String,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name)?;
        for (i, input) in self.inputs.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?
            }
            write!(f, "{}", input)?;
        }
        write!(f, ")")
    }
}

impl Function {
    #[allow(dead_code)]
    pub fn selector(&self) -> u32 {
        u32::from_be_bytes(self.selector_bytes())
    }

    pub fn selector_bytes(&self) -> [u8; 4] {
        let input = format!("{}", self);
        let bytes = Keccak256::digest(input);
        let mut result = [0u8; 4];
        result.copy_from_slice(&bytes[0..4]);
        result
    }
}

#[allow(dead_code)]
pub fn parse_str(json: &str) -> Result<Functions, serde_json::Error> {
    serde_json::from_str::<Vec<Function>>(json).map(Functions)
}

#[allow(dead_code)]
pub fn parse_bytes(json: &[u8]) -> Result<Functions, serde_json::Error> {
    serde_json::from_slice::<Vec<Function>>(json).map(Functions)
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
                    r#type: "int256".to_string(),
                    internal_type: Some("int256".to_string()),
                },
                Parameter {
                    name: "b".to_string(),
                    r#type: "int256".to_string(),
                    internal_type: Some("int256".to_string()),
                },
            ],
            outputs: vec![Parameter {
                name: "".to_string(),
                r#type: "int256".to_string(),
                internal_type: Some("int256".to_string()),
            }],
            state_mutability: StateMutability::Pure,
            r#type: "function".to_string(),
        }];
        assert_eq!(parse_str(MULTIPLY).unwrap().0, parsed);
    }

    #[test]
    fn test_display() {
        let funcs = parse_str(MULTIPLY).unwrap().0;
        let func = funcs.first().unwrap();
        assert_eq!(format!("{}", func), "multiply(int256,int256)");
    }

    #[test]
    fn test_selector() {
        // See: https://docs.soliditylang.org/en/develop/abi-spec.html#examples
        let baz_abi = r#"[
            {
                "name":"baz",
                "type":"function",
                "inputs":[
                    {"internalType":"uint32","name":"x","type":"uint32"},
                    {"internalType":"bool","name":"y","type":"bool"}
                ],
                "outputs":[
                    {"internalType":"bool","name":"r","type":"bool"}
                ],
                "stateMutability":"pure"
            }
        ]"#;
        let funcs = parse_str(baz_abi).unwrap().0;
        let func = funcs.first().unwrap();
        assert_eq!(func.selector(), 0xcdcd77c0);
    }
}
