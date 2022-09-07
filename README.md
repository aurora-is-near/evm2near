# EVM→NEAR

## Usage

```console
evm2near --help
evm2near < input.bin > output.wasm
```

## Development

### Prerequisites

- Rust toolchain (nightly 2022-09-07; macOS: `brew install rustup`)
- Solidity compiler `solc` (macOS: `brew install solidity`)
- `wasm-strip` (macOS: `brew install wabt`)

### Development Builds

```console
rustup target add wasm32-wasi
make evm2near
./evm2near --help
```

## Release

### Release Builds

```console
rustup target add wasm32-wasi
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
rustup target add aarch64-pc-windows-msvc
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
make clean all
```
