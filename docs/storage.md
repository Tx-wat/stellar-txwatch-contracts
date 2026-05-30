# Storage Reference

This document describes every storage key used by both contracts, its value type, storage tier (instance vs persistent), and TTL behavior.

---

## AlertRegistry

Source: `contracts/alert-registry/src/lib.rs`

### Storage Keys

| Key | Tier | Value Type | Description |
|---|---|---|---|
| `DataKey::Alert(id: u64)` | Persistent | `AlertConfig` | A single alert configuration, keyed by its numeric ID |
| `DataKey::AlertActive(id: u64)` | Persistent | `bool` | The `active` flag stored separately so it can be read without deserializing the full `AlertConfig` (see `get_alert_active`) |
| `DataKey::OwnerIndex(addr: Address)` | Persistent | `Vec<u64>` | List of alert IDs owned by a given address |
| `DataKey::ContractIndex(addr: Address)` | Persistent | `Vec<u64>` | List of alert IDs watching a given contract address |
| `symbol_short!("NEXT_ID")` | Instance | `u64` | Monotonic counter used to generate unique alert IDs |
| `symbol_short!("ADMIN")` | Instance | `Address` | Optional admin address that may remove alerts and set owner limits |
| `symbol_short!("LIMIT")` | Instance | `u32` | Optional per-owner active alert limit |

### AlertConfig Fields

| Field | Type | Description |
|---|---|---|
| `label` | `String` | Human-readable name for the alert (max 128 bytes) |
| `webhook_hash` | `String` | SHA-256 hex digest of the webhook URL |
| `rules` | `Vec<String>` | Rule descriptor strings (e.g. `"rule:transfer"`) |
| `owner` | `Address` | Address that owns and may mutate this alert |
| `target_contract` | `Address` | Contract address being watched |
| `created_at` | `u64` | Ledger timestamp at registration |
| `updated_at` | `u64` | Ledger timestamp of the most recent update |
| `active` | `bool` | Whether the alert is currently active |

### TTL Behavior

All four persistent key variants (`Alert`, `AlertActive`, `OwnerIndex`, `ContractIndex`) are extended by **100 ledgers** (≈ 8 minutes at 5 s/ledger) on every write that touches them.

| Function | Keys Extended |
|---|---|
| `register_alert` | `Alert(id)`, `AlertActive(id)`, `OwnerIndex(owner)`, `ContractIndex(target)` |
| `update_alert` | `Alert(id)`, `AlertActive(id)` |
| `update_webhook` | `Alert(id)` |
| `remove_alert` | Entries deleted — no TTL extension |

Read-only functions (`get_alert`, `get_alerts_for_contract`, `get_alerts_by_owner`, paginated variants, `get_alert_count`) do **not** extend any TTL.

The `NEXT_ID` instance key has no explicit TTL management — its lifetime is tied to the contract instance itself.

> See [docs/ttl.md](ttl.md) for implications of the 100-ledger setting and recommended production values.

---

## WatcherRegistry

Source: `contracts/watcher-registry/src/lib.rs`

### Storage Keys

| Key | Tier | Value Type | Description |
|---|---|---|---|
| `symbol_short!("ADMIN")` | Instance | `Address` | Current admin address |
| `symbol_short!("WATCHERS")` | Instance | `Vec<Address>` | List of authorized watcher node addresses |

### TTL Behavior

WatcherRegistry uses **instance storage exclusively**. Instance storage TTL is managed by the Stellar network and is not explicitly extended by any function in this contract. The TTL resets whenever the contract instance is accessed by any transaction that bumps the footprint.

There are no persistent storage entries in WatcherRegistry.

---

## Storage Tier Summary

| Contract | Tier | Keys | TTL Managed By |
|---|---|---|---|
| AlertRegistry | Persistent | `Alert`, `OwnerIndex`, `ContractIndex` | Contract (`extend_ttl`, 100 ledgers) |
| AlertRegistry | Instance | `NEXT_ID` | Network |
| WatcherRegistry | Instance | `ADMIN`, `WATCHERS` | Network |
