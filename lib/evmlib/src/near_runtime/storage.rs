use crate::state::{Word, WORD_SIZE};

const V1_KEY_SIZE: usize = 1 + WORD_SIZE;
const V1_VERSION_BYTE: u8 = 1;

pub enum StorageKey {
    V1([u8; V1_KEY_SIZE]),
}

impl StorageKey {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::V1(bytes) => bytes,
        }
    }

    pub fn from_word(word: Word) -> Self {
        let mut bytes = [0u8; V1_KEY_SIZE];
        bytes[0] = V1_VERSION_BYTE;
        bytes[1..].copy_from_slice(&word.to_be_bytes());
        Self::V1(bytes)
    }
}

impl AsRef<[u8]> for StorageKey {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
