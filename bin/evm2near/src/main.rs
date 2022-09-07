// This is free and unencumbered software released into the public domain.

mod analyze;
mod compile;
mod decode;
mod encode;

use clap::Parser;
use parity_wasm::elements::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
};

use crate::{compile::compile, decode::decode_bytecode};

#[derive(Parser, Debug)]
/// EVM to NEAR compiler
#[clap(name = "evm2near", version, about)]
struct Options {
    /// Define the chain ID
    #[clap(value_name = "ID", long, value_parser, default_value = "mainnet")]
    chain_id: String,

    #[clap(short = 'd', long, value_parser)]
    debug: bool,

    /// Disable precise EVM gas accounting
    #[clap(long = "fno-gas-accounting", value_parser)]
    no_gas_accounting: bool,

    /// Disable precise EVM program counter
    #[clap(long = "fno-program-counter", value_parser)]
    no_program_counter: bool,

    #[clap(value_name = "FILE", value_parser, default_value = "/dev/stdin")]
    input: PathBuf,

    #[clap(
        short = 'o',
        value_name = "FILE",
        value_parser,
        default_value = "/dev/stdout"
    )]
    output: PathBuf,

    #[clap(short = 'v', long, value_parser)]
    verbose: bool,

    #[clap(short = 'V', long, value_parser)]
    version: bool,
}

macro_rules! abort {
    ($($t:tt)*) => {{
        eprintln!($($t)*);
        std::process::exit(1)
    }};
}

fn main() -> impl std::process::Termination {
    let options = Options::parse_from(wild::args());
    if options.debug {
        eprintln!("{:?}", options);
    }

    let mut input = match options.input.to_str() {
        Some("/dev/stdin") | Some("-") => Box::new(stdin()) as Box<dyn Read>,
        _ => match File::open(&options.input) {
            Ok(file) => Box::new(file) as Box<dyn Read>,
            Err(err) => abort!(
                "Could not open input file `{}': {}",
                options.input.display(),
                err
            ),
        },
    };

    let mut input_buffer = String::new();
    match input.read_to_string(&mut input_buffer) {
        Ok(_) => {}
        Err(err) => abort!(
            "Could not read input file `{}': {}",
            options.input.display(),
            err
        ),
    };

    let input_program = match decode_bytecode(&input_buffer) {
        Err(err) => abort!("{}", err), // TODO
        Ok(program) => program,
    };
    if options.debug {
        eprintln!("{:?}", input_program.0);
    }

    let mut output = match options.output.to_str() {
        Some("/dev/stdout") | Some("-") => Box::new(stdout()) as Box<dyn Write>,
        _ => match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&options.output)
        {
            Ok(file) => Box::new(file) as Box<dyn Write>,
            Err(err) => abort!(
                "Could not open output file `{}': {}",
                options.output.display(),
                err
            ),
        },
    };

    let runtime_library = parity_wasm::deserialize_file("../evmlib/evmlib.wasi").unwrap();

    let output_program = compile(&input_program, runtime_library);
    if options.debug {
        eprintln!("{:?}", output_program); // TODO
    }

    output_program
        .serialize(&mut output)
        .expect("Failed to write module");
}
