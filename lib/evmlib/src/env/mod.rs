//! This module contains a trait for getting information about the execution
//! environment (eg block height, current address, etc). The purpose of abstracting
//! these calls into a trait is allowing us to provide mock values in tests, while
//! getting the real values using the NEAR host functions on-chain.

use crate::state::Word;

pub mod mock;

pub type Address = [u8; 20];
pub trait Env {
    /// Signature is &mut to allow for caching the result internally
    fn call_data(&mut self) -> &[u8];
    fn call_data_len(&self) -> usize;
    fn address(&self) -> Address;
    fn origin(&self) -> Address;
    fn caller(&self) -> Address;
    fn block_height(&self) -> u64;
    fn timestamp(&self) -> u64;
    fn storage_read(&mut self, key: Word) -> Word;
    fn storage_write(&mut self, key: Word, value: Word);
}