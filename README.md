# EVM → NEAR

`evm2near` is a project for compiling EVM bytecode into wasm bytecode, with the particular goal of having that wasm artifact be executable on the [NEAR blockchain](https://near.org/).
For ease of testing locally, `evm2near` also currently supports [wasi](https://wasi.dev/) as a target platform.
The wasi output can be run locally using a wasm runtime, for example [wasmtime](https://wasmtime.dev/).
This can be useful for debugging contracts without deploying to NEAR.

Even though `evm2near` is a general EVM bytecode to wasm bytecode transpiler, the CLI interface accepts a Solidity source file as input for convenience.
The source file is compiled to EVM bytecode using [solc](https://github.com/ethereum/solidity).
Using Solidity and `solc`, means `evm2near` also has access to the contract ABI.
This allows the output wasm artifact to contain functions that match the ones given in the contract.
For example, `test/calc.sol` contains a contract with a function `multiply(int a, int b)`, and the compiled wasm artifact will also contain a function called `multiply` which takes a JSON string as input.
The JSON input is expected to be an object with fields matching the function argument names (`a` and `b` in the example).
These functions generated based on the ABI are in addition to a general function called `execute`, which accepts binary input following the usual Solidity ABI (i.e. the first four bytes are the "selector" derived from the function signature, the remaining bytes are the input arguments encoded using Solidity's ABI format).

## Usage

### Compiling to wasi (for running locally)

```
evm2near INPUT_SOLIDITY_CONTRACT -o OUTPUT_WASM_FILE -b wasi
```

Example:

```console
evm2near test/calc.sol -o test.wasm -b wasi
```

Running the output in wasmtime:

```console
wasmtime --allow-unknown-exports test.wasm --invoke multiply -- '{"a":6, "b": 7}'
```

### Compiling to NEAR

```
evm2near INPUT_SOLIDITY_CONTRACT -o OUTPUT_WASM_FILE -b near
```

Example:

```console
evm2near test/calc.sol -o test.wasm -b near
```

Running the output using [near-cli](https://github.com/near/near-cli):

```console
near --networkId testnet dev-deploy test.wasm
near --networkId testnet --accountId $NEAR_ACCOUNT_ID call $DEV_CONTACT_ID multiply '{"a": 7, "b": 6}'
```

Note: you will need to set the value of `$DEV_CONTACT_ID` from the output of the prior `dev-deploy` command (you will see something like `Account id: dev-1663014663747-27418521013742` included in the output, then you would set `DEV_CONTACT_ID=dev-1663014663747-27418521013742`).

Note: you will need to use your own NEAR account for `$NEAR_ACCOUNT_ID`.
If you do not have one, you can create it using the [NEAR wallet](https://wallet.testnet.near.org/create), then access it via the CLI using the `near login` command.

### Help

```console
evm2near --help
```

## Development

### Prerequisites

- Rust toolchain (nightly 2022-09-07)
- Solidity compiler `solc` (0.8.16+)
- `wasm-strip` from WABT

#### Prerequisites on macOS

```console
brew install rustup solidity wabt
```

#### Prerequisites on Ubuntu

```console
curl -sSf https://sh.rustup.rs | sh

sudo apt-add-repository ppa:ethereum/ethereum
sudo apt update
sudo apt install solc

sudo apt install wabt
```

### Development Builds

```console
rustup target add wasm32-wasi
rustup target add wasm32-unknown-unknown
make
./evm2near --help
```

## Release

### Prerequisites

- Rust toolchain (nightly 2022-09-07)
- MinGW-w64 (10.0.0+)
- `wasm-strip` from WABT

#### Prerequisites on macOS

```console
brew install rustup mingw-w64 wabt
```

#### Prerequisites on Ubuntu

```console
curl -sSf https://sh.rustup.rs | sh

sudo apt install mingw-w64 wabt
sudo apt install gcc
```

### Release Builds

```console
rustup target add wasm32-wasi
rustup target add wasm32-unknown-unknown
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
rustup target add aarch64-pc-windows-msvc
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
make clean release
```
