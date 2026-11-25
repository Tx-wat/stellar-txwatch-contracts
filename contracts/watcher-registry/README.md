# WatcherRegistry

> Part of [stellar-txwatch-contracts](https://github.com/Tx-wat/stellar-txwatch-contracts)

On-chain registry for authorized watcher node addresses. An admin (or set of
admins) controls which off-chain watcher nodes are permitted to interact with
the `AlertRegistry` contract. Any caller can query whether a given address is
authorized.

This contract is a clean, self-contained example of common Soroban patterns
and is suitable for use as a reference implementation.

## Key functions

| Function | Description | Auth required |
| --- | --- | --- |
| `initialize(admin)` | Sets the initial admin set; fails if already initialized. | No (one-time init) |
| `add_admin(admin, new_admin)` | Adds `new_admin` to the admin set (idempotent). | Yes — `admin` |
| `remove_admin(admin, target)` | Removes `target` from the admin set; refuses to remove the last admin. | Yes — `admin` |
| `register_watcher(admin, watcher)` | Authorizes `watcher` (idempotent). | Yes — `admin` |
| `deregister_watcher(admin, watcher)` | Revokes authorization for `watcher`. | Yes — `admin` |
| `is_authorized(watcher)` | Returns whether `watcher` is authorized. | No (read-only) |
| `get_admins()` | Returns the current admin set. | No (read-only) |

## Storage schema

All state lives in **instance storage**, appropriate for a small,
frequently-accessed registry:

| Key | Type | Description |
| --- | --- | --- |
| `Admins` | `Vec<Address>` | Current admin set. |
| `Watchers` | `Map<Address, bool>` | Authorized watcher addresses. |

See [docs/storage.md](../../docs/storage.md) for storage and TTL details.

## Events emitted

Every privileged mutation emits a `soroban_sdk::events` entry so changes are
visible on-chain and indexable off-chain:

| Event | Topics | Data |
| --- | --- | --- |
| `admin_added` | `("admin_added",)` | `new_admin: Address` |
| `admin_removed` | `("admin_removed",)` | `target: Address` |
| `watcher_registered` | `("watcher_registered",)` | `watcher: Address` |
| `watcher_deregistered` | `("watcher_deregistered",)` | `watcher: Address` |

See [docs/events.md](../../docs/events.md) for the full event reference.

## Key patterns demonstrated

- **`require_auth()` for admin-gated mutations** — every state-changing
  function calls `admin.require_auth()` as its first line, delegating
  signature verification entirely to the Stellar protocol.
- **Multi-admin set** — the admin role is held by a `Vec<Address>` rather
  than a single address, eliminating the single-point-of-failure of a sole
  admin while keeping the authorization model simple and auditable.
- **Idempotent registration** — `register_watcher` and `add_admin` are
  no-ops when the address is already present, making them safe to call
  multiple times without side effects.
- **On-chain audit events** — every privileged mutation emits a
  `soroban_sdk::events` entry so changes are visible on-chain and
  indexable off-chain.
- **`#[contracterror]` enum** — typed error codes (`AlreadyInitialized`,
  `Unauthorized`, `NotInitialized`, `LastAdmin`) instead of raw panics,
  giving callers structured error handling.
- **Instance storage** — all state lives in instance storage, which is
  appropriate for a small, frequently-accessed registry.
- **Last-admin guard** — `remove_admin` refuses to remove the final admin,
  preventing the contract from becoming permanently unmanageable.

## Quick start

```bash
# Build
cargo build --release --target wasm32-unknown-unknown

# Test
cargo test
```

The test suite covers happy paths, unauthorized rejections, idempotency,
multi-admin operations, and event emission. See
[docs/testing.md](../../docs/testing.md) for the testing guide.

## Deployment

| Network | Contract ID |
| --- | --- |
| Testnet | `<TESTNET_CONTRACT_ID>` |

```bash
bash ../../scripts/deploy.sh --network testnet
```

## Invoke via Stellar CLI

```bash
# Initialize
stellar contract invoke \
  --id <CONTRACT_ID> --source <ADMIN_IDENTITY> --network testnet \
  -- initialize --admin <ADMIN_ADDRESS>

# Register a watcher
stellar contract invoke \
  --id <CONTRACT_ID> --source <ADMIN_IDENTITY> --network testnet \
  -- register_watcher --admin <ADMIN_ADDRESS> --watcher <WATCHER_ADDRESS>

# Check authorization
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet \
  -- is_authorized --watcher <WATCHER_ADDRESS>
```

## TypeScript bindings

```bash
npm install @tx-wat/watcher-registry @stellar/stellar-sdk
```

See [bindings/watcher-registry](../../bindings/watcher-registry/README.md).

## Documentation

- [docs/watcher-registry.md](../../docs/watcher-registry.md) — function reference
- [docs/storage.md](../../docs/storage.md) — storage schema and TTL
- [docs/events.md](../../docs/events.md) — event reference
- [docs/ttl.md](../../docs/ttl.md) — TTL management
- [docs/testing.md](../../docs/testing.md) — testing guide
- [docs/ecosystem-submission.md](../../docs/ecosystem-submission.md) — submission checklist

## License

MIT
