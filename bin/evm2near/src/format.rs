// This is free and unencumbered software released into the public domain.

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum InputFormat {
    Auto,
    Bin,
    Sol,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Auto,
    Wasm,
    Wat,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputABI {
    Near,
    Wasi,
}

#[allow(dead_code)]
pub fn parse_input_format(format: &str) -> Option<InputFormat> {
    use InputFormat::*;
    let result = match format {
        "auto" => Auto,
        "bytecode" | "bin" | "hex" => Bin,
        "solidity" | "sol" => Sol,
        _ => return None,
    };
    Some(result)
}

pub fn parse_input_extension(extension: Option<&str>) -> Option<InputFormat> {
    use InputFormat::*;
    let result = match extension.unwrap_or_default() {
        "bin" | "hex" => Bin,
        "sol" => Sol,
        _ => return None,
    };
    Some(result)
}

#[allow(dead_code)]
pub fn parse_output_format(format: &str) -> Option<OutputFormat> {
    use OutputFormat::*;
    let result = match format {
        "auto" => Auto,
        "wasm" => Wasm,
        "wat" => Wat,
        _ => return None,
    };
    Some(result)
}
