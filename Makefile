.PHONY: build test lint fmt deploy-testnet clean

build:
	cargo build --release --target wasm32-unknown-unknown

test:
	cargo test

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

deploy-testnet:
	bash scripts/deploy.sh

clean:
	cargo clean
