#!/usr/bin/env bash
# scripts/deploy.sh — Deploy both contracts to Stellar Testnet
set -euo pipefail

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
IDENTITY="${STELLAR_IDENTITY:-deployer}"

echo "==> Checking Stellar CLI..."
stellar --version

echo "==> Funding account on testnet..."
stellar keys generate --overwrite "$IDENTITY" --network "$NETWORK" 2>/dev/null || true
stellar keys fund "$IDENTITY" --network "$NETWORK"

echo "==> Building contracts..."
cargo build --release --target wasm32-unknown-unknown

ALERT_WASM="target/wasm32-unknown-unknown/release/alert_registry.wasm"
WATCHER_WASM="target/wasm32-unknown-unknown/release/watcher_registry.wasm"

echo "==> Deploying Alert Registry..."
ALERT_ID=$(stellar contract deploy \
  --wasm "$ALERT_WASM" \
  --source "$IDENTITY" \
  --network "$NETWORK")
echo "Alert Registry deployed: $ALERT_ID"

echo "==> Deploying Watcher Registry..."
WATCHER_ID=$(stellar contract deploy \
  --wasm "$WATCHER_WASM" \
  --source "$IDENTITY" \
  --network "$NETWORK")
echo "Watcher Registry deployed: $WATCHER_ID"

ADMIN_ADDRESS=$(stellar keys address "$IDENTITY")

echo "==> Initializing Watcher Registry..."
stellar contract invoke \
  --id "$WATCHER_ID" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  -- initialize \
  --admin "$ADMIN_ADDRESS"

echo ""
echo "==> Deployment complete. Update DEPLOYMENTS.md with:"
echo "    Alert Registry:   $ALERT_ID"
echo "    Watcher Registry: $WATCHER_ID"
