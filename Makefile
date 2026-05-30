.PHONY: build test lint fmt deploy-testnet clean bindings

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

bindings: build
	cd bindings/alert-registry && npm install && npm run build

clean:
	cargo clean
	rm -rf bindings/alert-registry/dist bindings/alert-registry/node_modules
