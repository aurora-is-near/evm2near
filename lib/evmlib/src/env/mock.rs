use crate::env::{Address, Env, EvmLog};
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
    pub logs: Vec<OwnedEvmLog>,
}

impl MockEnv {
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.call_data.clear();
        self.address = [0u8; 20];
        self.origin = [0u8; 20];
        self.caller = [0u8; 20];
        self.block_height = 0;
        self.timestamp = 0;
        self.storage = None;
        self.logs.clear();
    }
}

impl Env for MockEnv {
    fn call_data(&mut self) -> &[u8] {
        &self.call_data
    }

    fn call_data_len(&self) -> usize {
        self.call_data.len()
    }

    fn address(&mut self) -> Address {
        self.address
    }

    fn origin(&mut self) -> Address {
        self.origin
    }

    fn caller(&mut self) -> Address {
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

    fn log(&mut self, entry: EvmLog) {
        self.logs.push(entry.into());

        eprintln!("LOG {}", entry.to_json_string());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedEvmLog {
    pub address: Address,
    pub topics: Vec<Word>,
    pub data: Vec<u8>,
}

impl<'a> From<EvmLog<'a>> for OwnedEvmLog {
    fn from(log: EvmLog<'a>) -> Self {
        Self {
            address: log.address,
            topics: log.topics.to_vec(),
            data: log.data.to_vec(),
        }
    }
}
