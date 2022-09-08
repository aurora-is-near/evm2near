CARGO = cargo
WASM_STRIP = wasm-strip

all: evmlib.wasm evmlib.wasi

evmlib.wasm: Cargo.toml $(wildcard src/*.rs) Makefile
	$(CARGO) build --release --frozen --target=wasm32-unknown-unknown
	$(WASM_STRIP) ../../target/wasm32-unknown-unknown/release/$@
	ln -sf ../../target/wasm32-unknown-unknown/release/evmlib.wasm $@

evmlib.wasi: Cargo.toml $(wildcard src/*.rs) Makefile
	$(CARGO) build --release --frozen --target=wasm32-wasi
	$(WASM_STRIP) ../../target/wasm32-wasi/release/evmlib.wasm
	ln -sf ../../target/wasm32-wasi/release/evmlib.wasm $@

check:
	$(CARGO) test -- --nocapture --test-threads=1 --color=always

clean:
	$(CARGO) clean
	rm -f evmlib.wasm evmlib.wasi

.PHONY: check clean
