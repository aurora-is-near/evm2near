# EVM → NEAR

## Usage

```console
evm2near --help
evm2near < input.bin > output.wasm
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

apt-add-repository ppa:ethereum/ethereum
apt update
apt install solc

apt install wabt
```

### Development Builds

```console
rustup target add wasm32-wasi
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

apt install mingw-w64 wabt
```

### Release Builds

```console
rustup target add wasm32-wasi
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
rustup target add aarch64-pc-windows-msvc
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl
make clean release
```
