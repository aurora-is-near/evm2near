use crate::env::{Address, Env};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MockEnv {
    pub address: Address,
    pub origin: Address,
    pub caller: Address,
    pub block_height: u64,
    pub timestamp: u64,
}

impl Env for MockEnv {
    fn address(&self) -> Address {
        self.address
    }

    fn origin(&self) -> Address {
        self.origin
    }

    fn caller(&self) -> Address {
        self.caller
    }

    fn block_height(&self) -> u64 {
        self.block_height
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}
