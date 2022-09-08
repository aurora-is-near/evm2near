//! This module contains a trait for getting information about the execution
//! environment (eg block height, current address, etc). The purpose of abstracting
//! these calls into a trait is allowing us to provide mock values in tests, while
//! getting the real values using the NEAR host functions on-chain.

pub mod mock;

pub type Address = [u8; 20];
pub trait Env {
    fn address(&self) -> Address;
    fn origin(&self) -> Address;
    fn caller(&self) -> Address;
    fn block_height(&self) -> u64;
    fn timestamp(&self) -> u64;
}
