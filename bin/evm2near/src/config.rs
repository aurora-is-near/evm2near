// This is free and unencumbered software released into the public domain.

use std::{fs, path::PathBuf};

pub struct CompilerConfig {
    pub debug_path: Option<PathBuf>,
    pub optimize_level: u8,
    pub gas_accounting: bool,
    pub program_counter: bool,
    pub chain_id: u64,
}

impl CompilerConfig {
    pub fn new(
        debug_path: Option<PathBuf>,
        optimize_level: u8,
        gas_accounting: bool,
        program_counter: bool,
        chain_id: u64,
    ) -> Self {
        if let Some(debug_dir) = &debug_path {
            if fs::read_dir(debug_dir).is_ok() {
                fs::remove_dir_all(debug_dir).expect("unable to remove previous debug directory!");
            }
            fs::create_dir_all(debug_dir).expect("unable to create debug directory!");
        }
        CompilerConfig {
            debug_path,
            optimize_level,
            gas_accounting,
            program_counter,
            chain_id,
        }
    }
}
