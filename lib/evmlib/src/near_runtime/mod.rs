//! This module contains implementations of the various traits (Env, HashProvider, etc) using the
//! NEAR host functions.

use crate::env::{Address, Env};
use crate::hash_provider::HashProvider;
use crate::state::Word;
use std::collections::HashMap;

mod storage;

const KECCAK_REGISTER_ID: u64 = 1;
// This register can be safely used for all Env functions that get account_ids from the host
// because we always use the data before returning from the function, so it does not
// matter if it gets trashed later.
const ACCOUNT_REGISTER_ID: u64 = 2;
// The input must have its own register because we only set it once (as an optimization).
const INPUT_REGISTER_ID: u64 = 3;
const STORAGE_REGISTER_ID: u64 = 4;

pub struct NearRuntime {
    /// Cache for input from NEAR to prevent reading from the register multiple times.
    pub call_data: Option<Vec<u8>>,
    pub storage_cache: Option<HashMap<Word, Word>>,
    pub address_cache: Option<Address>,
    pub origin_cache: Option<Address>,
    pub caller_cache: Option<Address>,
}

impl HashProvider for NearRuntime {
    fn keccak256(input: &[u8]) -> [u8; 32] {
        unsafe {
            keccak256(
                input.len() as u64,
                input.as_ptr() as u64,
                KECCAK_REGISTER_ID,
            );
        }
        let mut host_result = [0u8; 32];
        Self::read_register_to_buffer(KECCAK_REGISTER_ID, &mut host_result);
        host_result
    }
}

impl Env for NearRuntime {
    fn call_data(&mut self) -> &[u8] {
        if self.call_data.is_none() {
            unsafe { input(INPUT_REGISTER_ID) };
            let host_result = Self::read_register(INPUT_REGISTER_ID);
            self.call_data = Some(host_result);
        }

        // Unwrap is clearly safe since we just set value to Some(..)
        self.call_data.as_ref().unwrap()
    }

    fn call_data_len(&self) -> usize {
        if let Some(call_data) = self.call_data.as_ref() {
            return call_data.len();
        }

        let mut host_result = unsafe { register_len(INPUT_REGISTER_ID) };

        // u64::MAX indicates the register is unused, therefore we need to load in input first.
        if host_result == u64::MAX {
            unsafe {
                input(INPUT_REGISTER_ID);
                host_result = register_len(INPUT_REGISTER_ID);
            }
        }

        // It should not be possible for the input to contain more than usize::MAX
        // bytes of data, but if it does then we'll take as much as we can.
        usize::try_from(host_result).unwrap_or(usize::MAX)
    }

    fn address(&mut self) -> Address {
        if let Some(address) = self.address_cache {
            return address;
        }

        let address = unsafe {
            current_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        };

        self.address_cache = Some(address);
        address
    }

    fn origin(&mut self) -> Address {
        if let Some(address) = self.origin_cache {
            return address;
        }

        let address = unsafe {
            signer_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        };

        self.origin_cache = Some(address);
        address
    }

    fn caller(&mut self) -> Address {
        if let Some(address) = self.caller_cache {
            return address;
        }

        let address = unsafe {
            predecessor_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        };

        self.caller_cache = Some(address);
        address
    }

    fn block_height(&self) -> u64 {
        unsafe { block_index() }
    }

    fn timestamp(&self) -> u64 {
        // NEAR gives timestamp in ns, but EVM expects seconds
        let ns = unsafe { block_timestamp() };
        ns / 1_000_000_000
    }

    fn storage_read(&mut self, key: Word) -> Word {
        if self.storage_cache.is_none() {
            self.storage_cache = Some(HashMap::new());
        }
        let storage_cache = self.storage_cache.as_mut().unwrap();

        if let Some(value) = storage_cache.get(&key) {
            return *value;
        }

        let storage_key = storage::StorageKey::from_word(key);
        let host_result = Self::inner_storage_read(storage_key.as_slice());
        let value = host_result
            .map(|bytes| Word::from_be_bytes(bytes.try_into().unwrap()))
            .unwrap_or(Word::ZERO);
        storage_cache.insert(key, value);
        value
    }

    fn storage_write(&mut self, key: Word, value: Word) {
        if self.storage_cache.is_none() {
            self.storage_cache = Some(HashMap::new());
        }
        // Unwrap is safe because we ensure it is Some(..) in the check above
        let previous_value = self.storage_cache.as_mut().unwrap().insert(key, value);
        // Storage needs to be updated if we have never seen this key/value pair before,
        // or if the value is different what from what it used to be.
        let need_to_update_storage = match previous_value {
            None => true,
            Some(x) if x != value => true,
            _ => false,
        };

        if need_to_update_storage {
            let storage_key = storage::StorageKey::from_word(key);
            let storage_value = value.to_be_bytes();
            Self::inner_storage_write(storage_key.as_slice(), &storage_value);
        }
    }

    fn log(&mut self, entry: crate::env::EvmLog) {
        let message = format!("LOG {}", entry.to_json_string());
        unsafe {
            log_utf8(message.len() as u64, message.as_ptr() as u64);
        }
    }
}

impl NearRuntime {
    fn register_len(register_id: u64) -> usize {
        let host_result = unsafe { register_len(register_id) };

        // By convention, an unused register will return a length of U64::MAX
        // (see https://nomicon.io/Proposals/bindings#registers).
        if host_result == u64::MAX {
            return 0;
        }

        // It should not be possible for a register to contain more than usize::MAX
        // bytes of data, but if it does then we'll take as much as we can.
        usize::try_from(host_result).unwrap_or(usize::MAX)
    }

    fn read_register(register_id: u64) -> Vec<u8> {
        let data_size = Self::register_len(register_id);
        let mut buffer = Vec::with_capacity(data_size);
        Self::read_register_to_buffer(register_id, &mut buffer);
        buffer
    }

    fn read_register_to_buffer(register_id: u64, buffer: &mut [u8]) {
        unsafe {
            read_register(register_id, buffer.as_ptr() as u64);
        }
    }

    fn inner_storage_read(key: &[u8]) -> Option<Vec<u8>> {
        let host_result =
            unsafe { storage_read(key.len() as u64, key.as_ptr() as u64, STORAGE_REGISTER_ID) };

        if host_result == 1 {
            Some(Self::read_register(STORAGE_REGISTER_ID))
        } else {
            None
        }
    }

    fn inner_storage_write(key: &[u8], value: &[u8]) {
        unsafe {
            storage_write(
                key.len() as u64,
                key.as_ptr() as u64,
                value.len() as u64,
                value.as_ptr() as u64,
                STORAGE_REGISTER_ID,
            );
        }
    }

    /// This function uses the data in `ACCOUNT_REGISTER_ID` as the input
    /// to the hash function which is used to derive the address. It is marked as
    /// unsafe to flag that register must be properly set before calling this function.
    unsafe fn account_id_to_address() -> Address {
        unsafe {
            // We can pass one register's value as the input to another host function
            // by specifying a length of u64::MAX. See https://nomicon.io/Proposals/bindings#registers
            keccak256(u64::MAX, ACCOUNT_REGISTER_ID, KECCAK_REGISTER_ID);
        }
        let mut hash = [0u8; 32];
        Self::read_register_to_buffer(KECCAK_REGISTER_ID, &mut hash);
        let mut result = [0u8; 20];
        result.copy_from_slice(&hash[12..32]);
        result
    }
}

extern "C" {
    fn read_register(register_id: u64, ptr: u64);
    fn register_len(register_id: u64) -> u64;

    fn current_account_id(register_id: u64);
    fn signer_account_id(register_id: u64);
    fn predecessor_account_id(register_id: u64);
    fn block_index() -> u64;
    fn block_timestamp() -> u64;
    fn input(register_id: u64);
    fn keccak256(value_len: u64, value_ptr: u64, register_id: u64);

    fn storage_write(
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64;
    fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64;

    fn log_utf8(len: u64, ptr: u64);
}
