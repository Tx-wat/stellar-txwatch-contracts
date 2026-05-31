#!/usr/bin/env bash
# scripts/verify.sh — Verify deployed contract WASM hash matches local build
# Usage: ./scripts/verify.sh --contract alert-registry|watcher-registry --contract-id <ID> [--network testnet|mainnet]
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
CONTRACT=""
CONTRACT_ID=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)     NETWORK="$2";     shift 2 ;;
    --contract)    CONTRACT="$2";    shift 2 ;;
    --contract-id) CONTRACT_ID="$2"; shift 2 ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
done

[[ -z "$CONTRACT" ]]    && { echo "Error: --contract required (alert-registry|watcher-registry)"; exit 1; }
[[ -z "$CONTRACT_ID" ]] && { echo "Error: --contract-id required"; exit 1; }

case "$NETWORK" in
  testnet)
    RPC_URL="https://soroban-testnet.stellar.org"
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
    ;;
  mainnet)
    RPC_URL="${MAINNET_RPC_URL:?MAINNET_RPC_URL must be set for mainnet}"
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
    ;;
  *)
    echo "Unsupported network: $NETWORK"; exit 1 ;;
esac

WASM_NAME="${CONTRACT//-/_}"
WASM="target/wasm32-unknown-unknown/release/${WASM_NAME}.wasm"

echo "==> Network: $NETWORK"
echo "==> Building contracts..."
cargo build --release --target wasm32-unknown-unknown

# Compute local WASM hash (sha256, hex only)
LOCAL_HASH=$(sha256sum "$WASM" | awk '{print $1}')
echo "==> Local WASM hash:    $LOCAL_HASH"

# Fetch deployed WASM hash via Stellar CLI
DEPLOYED_HASH=$(stellar contract info \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  | grep -i "wasm hash" | awk '{print $NF}' | tr -d '"')
echo "==> Deployed WASM hash: $DEPLOYED_HASH"

if [[ "$LOCAL_HASH" == "$DEPLOYED_HASH" ]]; then
  echo "==> MATCH: deployed contract matches local build."
else
  echo "==> MISMATCH: deployed contract does NOT match local build!"
  exit 1
fi
