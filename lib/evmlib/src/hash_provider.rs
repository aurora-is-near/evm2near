use sha3::{Digest, Keccak256};

pub trait HashProvider {
    fn keccak256(input: &[u8]) -> [u8; 32];
}

pub struct Native;

impl HashProvider for Native {
    fn keccak256(input: &[u8]) -> [u8; 32] {
        // Unwrap is safe because has is 256-bit
        Keccak256::digest(input).as_slice().try_into().unwrap()
    }
}
