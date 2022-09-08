CARGO = cargo
LIPO = lipo
WASM_STRIP = wasm-strip

all: evm2near

release:                   \
  evm2near-macos-arm       \
  evm2near-macos-x86       \
  evm2near-windows-arm.exe \
  evm2near-windows-x86.exe \
  evm2near-linux-arm       \
  evm2near-linux-x86

evm2near: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near
	ln -sf target/debug/evm2near evm2near

evm2near-macos: evm2near-macos-arm evm2near-macos-x86
	$(LIPO) -create -output $@ $^

evm2near-macos-arm: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near --release --frozen --target=aarch64-apple-darwin
	ln -sf target/aarch64-apple-darwin/release/evm2near $@

evm2near-macos-x86: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near --release --frozen --target=x86_64-apple-darwin
	ln -sf target/x86_64-apple-darwin/release/evm2near $@

evm2near-windows-arm.exe: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	#$(CARGO) build --package=evm2near --release --target=aarch64-pc-windows-msvc
	#ln -sf target/aarch64-pc-windows-msvc/release/evm2near.exe $@

evm2near-windows-x86.exe: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near --release --target=x86_64-pc-windows-gnu
	ln -sf target/x86_64-pc-windows-gnu/release/evm2near.exe $@

evm2near-linux-arm: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near --release --frozen --target=aarch64-unknown-linux-musl
	ln -sf target/aarch64-unknown-linux-musl/release/evm2near $@

evm2near-linux-x86: bin/evm2near/Cargo.toml $(wildcard bin/evm2near/src/*.rs) Makefile evmlib.wasi
	$(CARGO) build --package=evm2near --release --frozen --target=x86_64-unknown-linux-musl
	ln -sf target/x86_64-unknown-linux-musl/release/evm2near $@

evmlib.wasm: lib/evmlib/Cargo.toml $(wildcard lib/evmlib/src/*.rs) Makefile
	$(CARGO) build --package=evmlib --release --frozen --target=wasm32-unknown-unknown
	$(WASM_STRIP) target/wasm32-unknown-unknown/release/$@
	ln -sf target/wasm32-unknown-unknown/release/evmlib.wasm $@

evmlib.wasi: lib/evmlib/Cargo.toml $(wildcard lib/evmlib/src/*.rs) Makefile
	$(CARGO) build --package=evmlib --release --frozen --target=wasm32-wasi
	$(WASM_STRIP) target/wasm32-wasi/release/evmlib.wasm
	ln -sf target/wasm32-wasi/release/evmlib.wasm $@

check:
	$(CARGO) test -- --nocapture --test-threads=1 --color=always

clean:
	$(CARGO) clean
	rm -f evm2near evm2near-macos evm2near-*-* evmlib.wasm evmlib.wasi

.PHONY: check clean
