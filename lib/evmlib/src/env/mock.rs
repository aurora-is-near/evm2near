use crate::env::{Address, Env};
use crate::state::Word;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MockEnv {
    pub call_data: Vec<u8>,
    pub address: Address,
    pub origin: Address,
    pub caller: Address,
    pub block_height: u64,
    pub timestamp: u64,
    pub storage: Option<HashMap<Word, Word>>,
}

impl Env for MockEnv {
    fn call_data(&mut self) -> &[u8] {
        &self.call_data
    }

    fn call_data_len(&self) -> usize {
        self.call_data.len()
    }

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

    fn storage_read(&mut self, key: Word) -> Word {
        if self.storage.is_none() {
            self.storage = Some(HashMap::new());
        }

        self.storage
            .as_ref()
            .unwrap()
            .get(&key)
            .copied()
            .unwrap_or(crate::state::ZERO)
    }

    fn storage_write(&mut self, key: Word, value: Word) {
        if self.storage.is_none() {
            self.storage = Some(HashMap::new());
        }

        self.storage.as_mut().unwrap().insert(key, value);
    }
}
