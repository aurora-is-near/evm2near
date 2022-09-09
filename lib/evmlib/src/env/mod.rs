//! This module contains a trait for interacting with the execution environment.
//! This includes getters (eg block height, current address, etc), as well as state (ie storage)
//! access, logging, and exit functionality. The purpose of abstracting
//! these calls into a trait is allowing us to provide mock values in tests, while
//! getting the real values using the NEAR host functions on-chain.

use crate::state::Word;

pub mod mock;

pub type Address = [u8; 20];
pub trait Env {
    /// Signature is &mut to allow for caching the result internally
    fn call_data(&mut self) -> &[u8];
    fn call_data_len(&self) -> usize;
    fn address(&mut self) -> Address;
    fn origin(&mut self) -> Address;
    fn caller(&mut self) -> Address;
    fn block_height(&self) -> u64;
    fn timestamp(&self) -> u64;
    fn storage_read(&mut self, key: Word) -> Word;
    fn storage_write(&mut self, key: Word, value: Word);
    fn log(&mut self, entry: EvmLog);
    fn value_return(&mut self, return_data: &[u8]);
    fn revert(&mut self, return_data: &[u8]);
    /// Exit due to out of gas
    fn exit_oog(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvmLog<'a> {
    pub address: Address,
    pub topics: &'a [Word],
    pub data: &'a [u8],
}

impl<'a> EvmLog<'a> {
    pub fn to_json_string(self) -> String {
        let num_topics = self.topics.len();
        let topics_string = if num_topics == 0 {
            "[]".to_string()
        } else if num_topics == 1 {
            format!(r#"["0x{}"]"#, hex::encode(self.topics[0].to_be_bytes()))
        } else {
            format!(
                r#"["0x{}"{}]"#,
                hex::encode(self.topics[0].to_be_bytes()),
                self.topics[1..]
                    .iter()
                    .map(|t| format!(r#", "0x{}""#, hex::encode(t.to_be_bytes())))
                    .collect::<String>()
            )
        };
        format!(
            r#"{{ "address": "0x{}", "topics": {}, "data": "0x{}" }}"#,
            hex::encode(self.address),
            topics_string,
            hex::encode(self.data)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success,
    Revert,
    OutOfGas,
}
