//! This module contains implementations of the various traits (Env, HashProvider, etc) using the
//! NEAR host functions.

use crate::env::{Address, Env};
use crate::hash_provider::HashProvider;

const KECCAK_REGISTER_ID: u64 = 1;
// This register can be safely used for all Env functions that get account_ids from the host
// because we always use the data before returning from the function, so it does not
// matter if it gets trashed later.
const ACCOUNT_REGISTER_ID: u64 = 2;

pub struct NearRuntime;

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
    fn address(&self) -> Address {
        unsafe {
            current_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        }
    }

    fn origin(&self) -> Address {
        unsafe {
            signer_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        }
    }

    fn caller(&self) -> Address {
        unsafe {
            predecessor_account_id(ACCOUNT_REGISTER_ID);
            Self::account_id_to_address()
        }
    }

    fn block_height(&self) -> u64 {
        unsafe { block_index() }
    }

    fn timestamp(&self) -> u64 {
        // NEAR gives timestamp in ns, but EVM expects seconds
        let ns = unsafe { block_timestamp() };
        ns / 1_000_000_000
    }
}

impl NearRuntime {
    /*fn register_len(register_id: u64) -> usize {
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
    }*/

    fn read_register_to_buffer(register_id: u64, buffer: &mut [u8]) {
        unsafe {
            read_register(register_id, buffer.as_ptr() as u64);
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
    // fn register_len(register_id: u64) -> u64;

    fn current_account_id(register_id: u64);
    fn signer_account_id(register_id: u64);
    fn predecessor_account_id(register_id: u64);
    fn block_index() -> u64;
    fn block_timestamp() -> u64;
    // fn input(register_id: u64);
    fn keccak256(value_len: u64, value_ptr: u64, register_id: u64);
}
