# stellar-txwatch-contracts

[![codecov](https://codecov.io/gh/Tx-wat/stellar-txwatch-contracts/branch/main/graph/badge.svg)](https://codecov.io/gh/Tx-wat/stellar-txwatch-contracts)

Soroban smart contracts for on-chain alert configuration storage and watcher registry.  
Part of the [Tx-wat](https://github.com/Tx-wat) organization.

## Contracts

| Contract | Description |
|---|---|
| [Alert Registry](contracts/alert-registry/src/lib.rs) | Stores alert configs on-chain keyed by contract address |
| [Watcher Registry](contracts/watcher-registry/src/lib.rs) | Stores authorized watcher node addresses |

## WASM Size Budget

CI enforces a hard size budget of **200 KB (204800 bytes)** per contract WASM binary
(see [.github/workflows/wasm-size-check.yml](.github/workflows/wasm-size-check.yml)).
The build fails if any binary exceeds this threshold.

Current WASM sizes (reference). A change counts as **significant** when a contract binary grows or shrinks by more than **5 KB**; the PR author who causes that change updates this table in the same PR (the PR template includes a checklist item for it).

| Contract | WASM binary | Budget |
|---|---|---|
| Alert Registry | `alert_registry.wasm` — 69,464 bytes (67.8 KB) | 200 KB |
| Watcher Registry | `watcher_registry.wasm` — not measured: the crate does not compile on `main` (see note) | 200 KB |

> **Note:** Sizes were measured with `cargo build --release --target wasm32-unknown-unknown -p alert-registry` on Rust 1.88.0. The watcher-registry build currently fails in `contracts/watcher-registry/src/lib.rs` (`register_watcher` references `label`, `watchers`, and `String` without defining them). Update this table once that builds.

To check sizes locally after building:

```bash
cargo build --release --target wasm32-unknown-unknown
ls -l target/wasm32-unknown-unknown/release/*.wasm
```

## Quick Start

```bash
# Install Rust + Soroban target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Build
make build

# Test
make test

# Lint
make lint

# Lint (warnings are errors)
cargo clippy --all-targets --all-features -- -D warnings

# Generate TypeScript bindings
make bindings

# See all available targets
make help
```

## Deploying Contracts

Use `scripts/deploy.sh` to deploy both contracts to a target network. The script is
idempotent: if a contract is already deployed it skips the upload/deploy step and
runs a post-deploy health check to confirm the contract is callable.

Required environment variables (documented at the top of the script):

| Variable | Description |
|---|---|
| `NETWORK` | Target network: `testnet` or `mainnet` |
| `SOURCE` | Stellar CLI identity used to sign the deploy |
| `ALERT_REGISTRY_ID` | Existing AlertRegistry contract ID (optional; enables idempotency check) |
| `WATCHER_REGISTRY_ID` | Existing WatcherRegistry contract ID (optional; enables idempotency check) |

```bash
# Deploy to testnet
NETWORK=testnet SOURCE=my-identity ./scripts/deploy.sh

# Re-run against an existing deployment (skips upload, verifies health)
NETWORK=testnet SOURCE=my-identity \
  ALERT_REGISTRY_ID=<ALERT_REGISTRY_CONTRACT_ID> \
  WATCHER_REGISTRY_ID=<WATCHER_REGISTRY_CONTRACT_ID> \
  ./scripts/deploy.sh
```

## Test Coverage

CI measures test coverage with [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and uploads the report to [Codecov](https://codecov.io/gh/Tx-wat/stellar-txwatch-contracts). The build fails if overall coverage drops below 80%.

```bash
# Install and run locally
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --lcov --output-path lcov.info
```

Coverage targets:

| Crate | Target |
|---|---|
| alert-registry | >= 90% |
| watcher-registry | >= 90% |
| integration-tests | >= 80% |

## TypeScript Bindings

TypeScript bindings for the AlertRegistry contract are available on npm:

```bash
npm install @tx-wat/alert-registry-bindings
```

See [bindings/alert-registry/README.md](bindings/alert-registry/README.md) for usage examples.

To generate bindings locally:

```bash
make bindings
```

## Architecture

```mermaid
flowchart TD
    subgraph Stellar["Stellar Network (on-chain)"]
        AR["AlertRegistry\n─────────────\nstores alert configs\nkeyed by contract address"]
        WR["WatcherRegistry\n─────────────\nstores authorized\nwatcher addresses"]
    end

    subgraph OffChain["Off-chain (stellar-txwatch-core)"]
        W["Watcher Node\n─────────────\npolls Horizon\nmatches rules\nfires webhooks"]
    end

    Owner["Owner"] -->|"register_alert / update_alert"| AR
    Admin["Admin"] -->|"register_watcher / remove_watcher"| WR

    W -->|"is_authorized(watcher)"| WR
    W -->|"get_alerts_for_contract(target)"| AR
    Horizon["Horizon API"] -->|"GET /accounts/{id}/transactions"| W
    W -->|"POST webhook URL"| Endpoint["Downstream\nIntegration"]
```

**Data flow:**

1. An owner registers an alert in `AlertRegistry` — specifying the target contract, rules, and a hashed webhook URL.
2. Authorized watcher nodes are recorded in `WatcherRegistry` by an admin.
3. A watcher node polls Horizon for transaction activity, fetches matching alert configs from `AlertRegistry`, and checks whether any rule matches.
4. On a match the watcher fires the configured webhook so downstream integrations can react.

---

## How it works

The system is centered around three data-flow steps:

1. An owner registers an alert in `AlertRegistry` with the contract address, labels, webhook hash, and rules.
2. Authorized watcher nodes poll Horizon for transaction activity, then check the stored alert definitions in `AlertRegistry` to determine whether a watched contract event matches.
3. When a match is found, the watcher fires the configured webhook so downstream integrations can react.

This keeps alert configuration on-chain while letting watcher nodes perform off-chain polling and delivery.

## Stellar Integration

### Network Configuration

```toml
# Testnet
rpc_url       = "https://soroban-testnet.stellar.org"
passphrase    = "Test SDF Network ; September 2015"
horizon_url   = "https://horizon-testnet.stellar.org"

# Mainnet
rpc_url       = "https://mainnet.stellar.validationcloud.io/v1/<API_KEY>"
passphrase    = "Public Global Stellar Network ; September 2015"
horizon_url   = "https://horizon.stellar.org"
```

### Invoking Contracts (Stellar CLI)

```bash
# Register a watcher (admin only)
stellar contract invoke \
  --id <WATCHER_REGISTRY_CONTRACT_ID> \
  --source <ADMIN_IDENTITY> \
  --network testnet \
  -- register_watcher \
  --admin <ADMIN_ADDRESS> \
  --watcher <WATCHER_ADDRESS>

# Register an alert config
stellar contract invoke \
  --id <ALERT_REGISTRY_CONTRACT_ID> \
  --source <OWNER_IDENTITY> \
  --network testnet \
  -- register_alert \
  --owner <OWNER_ADDRESS> \
  --target_contract <WATCHED_CONTRACT_ADDRESS> \
  --label "My Alert" \
  --webhook_hash "<sha256-of-webhook-url>" \
  --rules '["rule:transfer","rule:mint"]'

# Query alerts for a contract
stellar contract invoke \
  --id <ALERT_REGISTRY_CONTRACT_ID> \
  --network testnet \
  -- get_alerts_for_contract \
  --target_contract <WATCHED_CONTRACT_ADDRESS>
```

### Invoking Contracts (JavaScript SDK)

```js
import {
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  BASE_FEE,
  nativeToScVal,
  Address,
} from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const contract = new Contract("<ALERT_REGISTRY_CONTRACT_ID>");

// Build a register_alert transaction
const account = await server.getAccount(ownerKeypair.publicKey());
const tx = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(
    contract.call(
      "register_alert",
      new Address(ownerKeypair.publicKey()).toScVal(),          // owner
      new Address("<WATCHED_CONTRACT_ADDRESS>").toScVal(),      // target_contract
      nativeToScVal("My Alert", { type: "string" }),            // label
      nativeToScVal("<sha256-of-webhook-url>", { type: "string" }), // webhook_hash
      nativeToScVal(["rule:transfer", "rule:mint"], { type: "array", element: { type: "string" } }), // rules
    )
  )
  .setTimeout(30)
  .build();

const preparedTx = await server.prepareTransaction(tx);
preparedTx.sign(ownerKeypair);
const result = await server.sendTransaction(preparedTx);
console.log("Transaction hash:", result.hash);
```

```js
import {
  Contract,
  SorobanRpc,
  TransactionBuilder,
  Networks,
  BASE_FEE,
  Address,
} from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const contract = new Contract("<WATCHER_REGISTRY_CONTRACT_ID>");

// Initialize the registry (one-time, admin only)
const account = await server.getAccount(adminKeypair.publicKey());
const initTx = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(
    contract.call(
      "initialize",
      new Address(adminKeypair.publicKey()).toScVal(), // admin
    )
  )
  .setTimeout(30)
  .build();

const preparedInit = await server.prepareTransaction(initTx);
preparedInit.sign(adminKeypair);
await server.sendTransaction(preparedInit);

// Register a watcher node (admin only)
const account2 = await server.getAccount(adminKeypair.publicKey());
const registerTx = new TransactionBuilder(account2, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(
    contract.call(
      "register_watcher",
      new Address(adminKeypair.publicKey()).toScVal(),   // admin
      new Address("<WATCHER_NODE_ADDRESS>").toScVal(),   // watcher
    )
  )
  .setTimeout(30)
  .build();

const preparedRegister = await server.prepareTransaction(registerTx);
preparedRegister.sign(adminKeypair);
await server.sendTransaction(preparedRegister);

// Check if an address is an authorized watcher (read-only, no signature needed)
const account3 = await server.getAccount(adminKeypair.publicKey());
const checkTx = new TransactionBuilder(account3, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(
    contract.call(
      "is_authorized",
      new Address("<WATCHER_NODE_ADDRESS>").toScVal(), // watcher
    )
  )
  .setTimeout(30)
  .build();

const result = await server.simulateTransaction(checkTx);
console.log("Is authorized:", result.result?.retval); // SCV_BOOL
```

### Invoking Contracts (Rust SDK)

```rust
use soroban_sdk::{Address, Env, String, Vec};

// In a cross-contract call context:
let alert_registry = AlertRegistryClient::new(&env, &alert_registry_id);
let config_id = alert_registry.register_alert(
    &owner,
    &target_contract,
    &String::from_str(&env, "My Alert"),
    &String::from_str(&env, "<webhook-hash>"),
    &rules,
);
```

> **Re-entrancy safety:** Soroban executes contract calls atomically and prevents classic callback-based re-entrancy within the same transaction. The registry contracts only mutate local storage after `require_auth()` succeeds, and they do not invoke external contracts, so re-entrancy is not a concern for these registries.

## Handsoff notes

<!-- handsoff-issue-394 -->
- #394: test: add test for get_active_alerts_for_contract excludes inactive alerts

<!-- handsoff-issue-397 -->
- #397: test: add test for get_alerts_modified_since returns only alerts updated at or after timestamp
