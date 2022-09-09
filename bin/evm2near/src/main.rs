// This is free and unencumbered software released into the public domain.

mod analyze;
mod compile;
mod config;
mod decode;
mod encode;
mod error;
mod format;
mod solidity;

use clap::Parser;
use parity_wasm::elements::Serialize;
use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
};

use crate::{
    compile::compile,
    config::CompilerConfig,
    decode::decode_bytecode,
    format::{parse_input_extension, InputFormat, OutputFormat},
    solidity::SOLC,
};

#[derive(Parser, Debug)]
/// EVM to NEAR compiler
#[clap(name = "evm2near", version, about)]
struct Options {
    /// The chain ID
    #[clap(value_name = "ID", long, value_parser, default_value = "mainnet")]
    chain_id: String,

    /// Enable debugging
    #[clap(short = 'd', long, value_parser)]
    debug: bool,

    /// The input format
    #[clap(short = 'f', long, value_parser, default_value = "auto")]
    from: InputFormat,

    /// Disable precise EVM gas accounting
    #[clap(long = "fno-gas-accounting", value_parser)]
    no_gas_accounting: bool,

    /// Disable precise EVM program counter
    #[clap(long = "fno-program-counter", value_parser)]
    no_program_counter: bool,

    /// The input file
    #[clap(value_name = "FILE", value_parser, default_value = "/dev/stdin")]
    input: PathBuf,

    /// The output file
    #[clap(
        short = 'o',
        value_name = "FILE",
        value_parser,
        default_value = "/dev/stdout"
    )]
    output: PathBuf,

    /// The output format
    #[clap(short = 't', long, value_parser, default_value = "auto")]
    to: OutputFormat,

    /// Enable verbose output
    #[clap(short = 'v', long, value_parser)]
    verbose: bool,

    /// Print the version and exit
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

    let input_path = options.input.as_path();
    let input_ext = input_path.extension().and_then(OsStr::to_str);
    let input_format = match options.from {
        InputFormat::Auto => match parse_input_extension(input_ext) {
            Some(format) => format,
            None => InputFormat::Bin, // the default
        },
        format => format,
    };

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

    let input_program = match input_format {
        InputFormat::Auto | InputFormat::Bin => {
            match decode_bytecode(&input_buffer) {
                Err(err) => abort!("{}", err), // TODO
                Ok(program) => program,
            }
        }
        InputFormat::Sol => match solidity::compile(input_path) {
            Ok(program) => program,
            Err(err) => abort!(
                "Failed to compile {} code: {}",
                "Solidity",
                err.with_program(SOLC)
            ),
        },
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

    let runtime_library = parity_wasm::deserialize_file("evmlib.wasi").unwrap(); // FIXME

    let output_program = compile(
        &input_program,
        runtime_library,
        CompilerConfig {
            gas_accounting: !options.no_gas_accounting,
            program_counter: !options.no_program_counter,
            chain_id: match options.chain_id.as_str() {
                "mainnet" => 1313161554,
                "testnet" => 1313161555,
                "betanet" => 1313161556,
                s => match s.parse::<u64>() {
                    Ok(n) => n,
                    Err(err) => abort!("Could not parse `{}': {}", s, err),
                },
            },
        },
    );
    if options.debug {
        eprintln!("{:?}", output_program); // TODO
    }

    output_program
        .serialize(&mut output)
        .expect("Failed to write module");
}
