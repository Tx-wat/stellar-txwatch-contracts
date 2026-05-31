# Event Schema

This document describes the planned on-chain events emitted by the `AlertRegistry` and `WatcherRegistry` contracts. Events are not yet fully implemented across all paths, but this spec defines the intended topics and data shapes so contributors have a clear target.

---

## AlertRegistry Events

### `alert/register`

Emitted when a new alert is successfully registered.

| Field | Type | Description |
|---|---|---|
| `topic[0]` | `Symbol` | `"alert"` |
| `topic[1]` | `Symbol` | `"register"` |
| `data[0]` | `u64` | The newly assigned alert ID |
| `data[1]` | `Address` | The owner address |
| `data[2]` | `Address` | The target contract address being watched |

**Example (Rust SDK)**

```rust
env.events().publish(
    (symbol_short!("alert"), symbol_short!("register")),
    (id, owner, target_contract),
);
```

---

### `alert/remove`

Emitted when an alert is removed, either by its owner or by an admin.

| Field | Type | Description |
|---|---|---|
| `topic[0]` | `Symbol` | `"alert"` |
| `topic[1]` | `Symbol` | `"remove"` |
| `data[0]` | `u64` | The ID of the removed alert |
| `data[1]` | `Address` | The caller who triggered the removal |

**Example (Rust SDK)**

```rust
env.events().publish(
    (symbol_short!("alert"), symbol_short!("remove")),
    (config_id, caller),
);
```

---

## WatcherRegistry Events

> Planned — not yet emitted. The shapes below are the intended design.

### `watcher/add`

Emitted when a new watcher address is registered.

| Field | Type | Description |
|---|---|---|
| `topic[0]` | `Symbol` | `"watcher"` |
| `topic[1]` | `Symbol` | `"add"` |
| `data[0]` | `Address` | The watcher address that was added |
| `data[1]` | `Address` | The admin who authorized the addition |

---

### `watcher/remove`

Emitted when a watcher address is deregistered.

| Field | Type | Description |
|---|---|---|
| `topic[0]` | `Symbol` | `"watcher"` |
| `topic[1]` | `Symbol` | `"remove"` |
| `data[0]` | `Address` | The watcher address that was removed |
| `data[1]` | `Address` | The admin who authorized the removal |

---

## Notes

- All topics use `symbol_short!` which limits each symbol to 9 bytes. Keep topic strings within that limit.
- Event data is encoded as a tuple. Consumers should decode positionally.
- Soroban events are queryable via Horizon's `/transactions/{id}/effects` endpoint or the `stellar-sdk` event streaming API.
- See [Soroban Events docs](https://developers.stellar.org/docs/learn/smart-contract-internals/events) for details on subscribing to and filtering events.
