.PHONY: help build test lint fmt fmt-check check doc deploy-testnet bindings clean verify

# Print available targets with descriptions.
help:
	@echo "Available targets:"
	@echo "  build          Build the contract (cargo build --release --target wasm32-unknown-unknown)"
	@echo "  test           Run the test suite (cargo test --all)"
	@echo "  lint           Run clippy with warnings as errors (cargo clippy --all-targets -- -D warnings)"
	@echo "  fmt            Format all code (cargo fmt --all)"
	@echo "  fmt-check      Check formatting without modifying files (cargo fmt --all -- --check)"
	@echo "  check          Run fmt-check, clippy and tests in sequence (pre-PR check)"
	@echo "  doc            Build and open documentation (cargo doc --no-deps --open)"
	@echo "  deploy-testnet Deploy the contract to testnet (scripts/deploy.sh)"
	@echo "  bindings       Generate TypeScript bindings for WatcherRegistry"
	@echo "  clean          Remove build artifacts (cargo clean)"
	@echo "  verify         Run verification against testnet (scripts/verify.sh --network testnet)"

# Build the contract for the wasm32 target in release mode.
build:
	cargo build --release --target wasm32-unknown-unknown

# Run the full test suite across all targets.
test:
	cargo test --all

# Run clippy across all targets, treating warnings as errors.
lint:
	cargo clippy --all-targets -- -D warnings

# Format all code.
fmt:
	cargo fmt --all

# Check formatting without modifying files.
fmt-check:
	cargo fmt --all -- --check

# Run formatting check, clippy (warnings as errors) and tests in sequence.
check:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
	cargo test --all

# Build and open the crate documentation.
doc:
	cargo doc --no-deps --open

# Deploy the contract to testnet.
deploy-testnet:
	bash scripts/deploy.sh

# Generate TypeScript bindings for WatcherRegistry.
# Requires: stellar CLI on PATH and a prior `make build`.
# Usage: CONTRACT_ID=CXXX... make bindings
bindings: build
	stellar contract bindings typescript \
		--wasm target/wasm32-unknown-unknown/release/watcher_registry.wasm \
		--contract-id $(CONTRACT_ID) \
		--output-dir bindings/watcher-registry \
		--overwrite
	cd bindings/watcher-registry && npm install && npm run build

# Remove build artifacts.
clean:
	cargo clean
	rm -rf bindings/watcher-registry/dist bindings/watcher-registry/node_modules

# Run verification against testnet.
verify:
	bash scripts/verify.sh --network testnet
