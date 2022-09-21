// This is free and unencumbered software released into the public domain.

pub struct CompilerConfig {
    pub debug: bool,
    pub optimize_level: u8,
    pub gas_accounting: bool,
    pub program_counter: bool,
    pub chain_id: u64,
}
