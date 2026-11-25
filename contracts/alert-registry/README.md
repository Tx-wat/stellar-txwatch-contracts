# AlertRegistry

> Part of [stellar-txwatch-contracts](https://github.com/Tx-w/stellar-txwatch-contracts)

On-chain registry for alert configurations. Each alert specifies a target
contract address to watch, a set of rule descriptors, and a SHA-256 hash of
the destination webhook URL. Off-chain watcher nodes read these configs to
decide when and where to fire notifications.

This contract demonstrates several intermediate-to-advanced Soroban patterns
and is suitable for use as a reference implementation.

## Key patterns demonstrated

- **Persistent storage with TTL extension** — alert configs and their indexes
  are stored in persistent storage and have their TTL extended on every write,
  keeping entries alive as long as they are actively used.
- **Dual-index lookups** — alerts are indexed by both owner address
  (`OwnerIndex`) and target contract address (`ContractIndex`), enabling
  efficient queries in both dimensions without a full scan.
- **Paginated query variants** — `get_contract_alerts_paginated` and
  `get_alerts_by_owner_paginated` accept `offset` and `limit` parameters,
  preventing instruction-limit overruns for owners with many alerts.
- **Monotonic ID counter** — a `u64` counter in instance storage generates
  unique, sequential alert IDs without requiring external coordination.
- **Privacy-preserving webhook storage** — only the SHA-256 hex digest of the
  webhook URL is stored on-chain; the raw URL never appears in contract state.
- **Rule descriptor validation** — the contract validates each rule string
  against a known allowlist (`rule:transfer`, `rule:mint`) before accepting it,
  preventing garbage data from entering the registry.
- **Per-owner rate limiting** — an optional admin-configurable limit caps the
  number of active alerts per owner, enforced at registration time.
- **Admin role with transfer** — an optional admin address can remove any
  alert and set owner limits; the role can be transferred via `transfer_admin`.

## Key functions

| Function | Description | Auth required |
| --- | --- | --- |
| `register_alert` | Register a new alert for a target contract with rules and webhook hash | Owner |
| `remove_alert` | Remove an alert by ID | Owner or admin |
| `set_alert_active` | Toggle only the `active` flag of an alert; rules, label, webhook and tags are unchanged | Owner |
| `get_alert` | Fetch a single alert config by ID | None |
| `get_contract_alerts` | List all alerts watching a target contract | None |
| `get_contract_alerts_paginated` | Paginated list of alerts for a target contract | None |
| `get_alerts_by_owner` | List all alerts owned by an address | None |
| `get_alerts_by_owner_paginated` | Paginated list of alerts for an owner | None |
| `get_alert_ids_by_owner_paginated` | One page of live alert IDs for an owner; `limit` capped at `MAX_BATCH_SIZE` | None |
| `get_alert_count_for_contract` | Number of live alerts watching a target contract (`u32`, TypeScript `number`) | None |
| `set_owner_limit` | Set the max active alerts per owner | Admin |
| `transfer_admin` | Transfer the admin role to a new address | Admin |

## Storage schema

- **Instance storage** — `Admin` (optional admin address), `AlertCounter`
  (monotonic `u64` ID counter), and `OwnerLimit` (optional per-owner cap).
- **Persistent storage** — `Alert(id)` config records, `OwnerIndex(owner)`
  and `ContractIndex(target_contract)` index lists. TTLs are extended on every
  write to keep active entries alive.

See [docs/storage.md](../../docs/storage.md) and [docs/ttl.md](../../docs/ttl.md)
for the full storage and TTL model.

## Events emitted

- `alert_registered` — emitted when a new alert is created (includes alert ID,
  owner, and target contract).
- `alert_removed` — emitted when an alert is removed.
- `admin_transferred` — emitted when the admin role changes.

See [docs/events.md](../../docs/events.md) for the event schema.

## Quick start

```bash
cargo build --release --target wasm32-unknown-unknown
cargo test
```

See [docs/testing.md](../../docs/testing.md) for the testing strategy.

## Deploy (testnet)

```bash
bash ../../scripts/deploy.sh --network testnet
```

Testnet contract ID: `<ALERT_REGISTRY_TESTNET_CONTRACT_ID>`

## Invoke via Stellar CLI

```bash
# Register an alert
stellar contract invoke \
  --id <CONTRACT_ID> --source <OWNER_IDENTITY> --network testnet \
  -- register_alert \
  --owner <OWNER_ADDRESS> \
  --target_contract <WATCHED_CONTRACT> \
  --label "My Alert" \
  --webhook_hash "<sha256-hex>" \
  --rules '["rule:transfer"]'

# Query alerts for a contract (paginated)
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet \
  -- get_contract_alerts_paginated \
  --target_contract <WATCHED_CONTRACT> \
  --offset 0 \
  --limit 20
```

## Function reference

See [docs/alert-registry.md](../../docs/alert-registry.md).

## License

MIT
