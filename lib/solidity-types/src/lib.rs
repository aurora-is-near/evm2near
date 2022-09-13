// This is free and unencumbered software released into the public domain.

use serde::Deserialize;
use std::fmt;

pub use ethabi;

pub fn parse_type(s: &str) -> Option<ValueType> {
    match s {
        "address" => Some(ValueType::Address),
        "bytes" => Some(ValueType::Bytes),
        "bytes32" => Some(ValueType::Bytes32),
        "bool" => Some(ValueType::Bool),
        "int8" => Some(ValueType::Int8),
        "int32" => Some(ValueType::Int32),
        "int256" => Some(ValueType::Int256),
        "string" => Some(ValueType::String),
        "uint8" => Some(ValueType::Uint8),
        "uint32" => Some(ValueType::Uint32),
        "uint256" => Some(ValueType::Uint256),
        _ => None,
    }
}

/// See: https://docs.soliditylang.org/en/v0.8.16/types.html
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all(deserialize = "camelCase"))]
pub enum ValueType {
    Address,
    AddressPayable,
    Bytes,
    Bytes32,
    Bool,
    Function,
    Int8,
    Int32,
    Int256,
    String,
    Uint8,
    Uint32,
    Uint256,
}

impl ValueType {
    pub fn as_param_type(&self) -> Option<ethabi::ParamType> {
        match self {
            ValueType::Address => Some(ethabi::ParamType::Address),
            ValueType::AddressPayable => None,
            ValueType::Bytes => Some(ethabi::ParamType::Bytes),
            ValueType::Bytes32 => Some(ethabi::ParamType::FixedBytes(32)),
            ValueType::Bool => Some(ethabi::ParamType::Bool),
            ValueType::Function => None,
            ValueType::Int8 => Some(ethabi::ParamType::Int(8)),
            ValueType::Int32 => Some(ethabi::ParamType::Int(32)),
            ValueType::Int256 => Some(ethabi::ParamType::Int(256)),
            ValueType::String => Some(ethabi::ParamType::String),
            ValueType::Uint8 => Some(ethabi::ParamType::Uint(8)),
            ValueType::Uint32 => Some(ethabi::ParamType::Uint(32)),
            ValueType::Uint256 => Some(ethabi::ParamType::Uint(256)),
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ValueType::*;
        match self {
            Address => write!(f, "address"),
            AddressPayable => write!(f, "address payable"),
            Bytes => write!(f, "bytes"),
            Bytes32 => write!(f, "bytes32"),
            Bool => write!(f, "bool"),
            Function => write!(f, "function"),
            Int8 => write!(f, "int8"),
            Int32 => write!(f, "int32"),
            Int256 => write!(f, "int256"),
            String => write!(f, "string"),
            Uint8 => write!(f, "uint8"),
            Uint32 => write!(f, "uint32"),
            Uint256 => write!(f, "uint256"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_type() {
        assert_eq!(super::parse_type("int256"), Some(super::ValueType::Int256))
    }
}
