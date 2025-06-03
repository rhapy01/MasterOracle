.PHONY: build check clean fmt

clean:
	cargo clean

fmt:
	cargo +nightly fmt --all

fmt-check:
	cargo +nightly fmt --all -- --check

check:
	cargo clippy --all-features --locked -- -D warnings

build:
	cargo build --target wasm32-wasi --profile release-wasm

install-tools:
	bun install