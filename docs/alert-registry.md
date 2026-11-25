# Alert Registry — Function Reference

Contract that stores alert configurations on-chain, keyed by contract address.

---

## Data Types

### `AlertConfig`

| Field | Type | Description |
|---|---|---|
| `label` | `String` | Human-readable name for the alert |
| `webhook_hash` | `String` | SHA-256 hex digest of the webhook URL (privacy-preserving; see [Webhook Hash Scheme](#webhook-hash-scheme) below) |
| `rules` | `Vec<String>` | Serialized rule descriptors |
| `owner` | `Address` | Address that owns this config |
| `target_contract` | `Address` | Contract being watched |
| `created_at` | `u64` | Ledger timestamp at creation |
| `updated_at` | `u64` | Ledger timestamp of last update |
| `active` | `bool` | Whether the alert is active |
| `pending_webhook_hash` | `Option<String>` | Pending webhook hash proposed via `propose_webhook`, not yet confirmed. `None` when no rotation is in progress. |

---

## Webhook Hash Scheme

The `webhook_hash` field stores a **SHA-256 hex digest** of the destination webhook URL. The raw URL is never written on-chain, which prevents publicly exposing private endpoint addresses.

### Algorithm

| Property | Value |
|---|---|
| Hash function | SHA-256 |
| Encoding | Lowercase hex string (64 characters) |
| Input | The raw webhook URL, UTF-8 encoded, no trailing newline |

### Computing the Hash

**Shell (openssl):**
```bash
echo -n "https://example.com/my-webhook" | openssl dgst -sha256
# SHA2-256(stdin)= 6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b
```

**Shell (sha256sum):**
```bash
printf '%s' 'https://example.com/my-webhook' | sha256sum
# 6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b  -
```

**JavaScript:**
```js
const hash = await crypto.subtle.digest(
  "SHA-256",
  new TextEncoder().encode("https://example.com/my-webhook"),
);
const hex = Array.from(new Uint8Array(hash))
  .map((b) => b.toString(16).padStart(2, "0"))
  .join("");
```

**Python:**
```python
import hashlib
url = "https://example.com/my-webhook"
hex_digest = hashlib.sha256(url.encode()).hexdigest()
```

**Rust:**
```rust
use sha2::{Digest, Sha256};
let hex_digest = format!("{:x}", Sha256::digest(b"https://example.com/my-webhook"));
```

### Verification

Off-chain watcher nodes store the original webhook URL locally and verify against the on-chain hash before firing a delivery. A mismatch indicates tampering or an out-of-date local config.

To rotate a webhook URL, use `update_webhook` with the new SHA-256 hex digest and update your local watcher config to match.

---

## Webhook Rotation

Rotating an alert's webhook uses a two-step flow. The new hash is staged in `AlertConfig.pending_webhook_hash` first, and the live `webhook_hash` changes only when the owner confirms. This avoids a window where the old webhook is removed before the new one is confirmed by the off-chain watcher.

### Lifecycle of `pending_webhook_hash`

| State | `webhook_hash` | `pending_webhook_hash` | Set by |
|---|---|---|---|
| Registered, no rotation in progress | `H1` | `None` | `register_alert` |
| Rotation proposed | `H1` (unchanged) | `Some(H2)` | `propose_webhook` |
| Proposal overwritten | `H1` (unchanged) | `Some(H3)` | second `propose_webhook` |
| Rotation confirmed | `H3` | `None` | `confirm_webhook` |

Step by step:

1. **Propose.** The owner calls `propose_webhook(caller, config_id, new_webhook_hash)`. The contract stores the hash in `pending_webhook_hash`, updates `updated_at`, and emits `(alert, wh_prop)` with data `(config_id, caller)`. The live `webhook_hash` is not changed, so the watcher keeps posting to the current URL.
2. **Overwrite (optional).** A second `propose_webhook` replaces the pending value. Only the most recent proposal is kept, and the earlier one is lost.
3. **Confirm.** The owner calls `confirm_webhook(caller, config_id)`. The contract takes the pending value, writes it to `webhook_hash`, clears `pending_webhook_hash`, and emits `(alert, webhook)` with data `(config_id, caller)`.

### Edge cases

- **Confirm with nothing pending** returns `NoPendingWebhook`. The stored state does not change.
- **Propose or confirm by a non-owner** returns `Unauthorized`. The owner check runs after `require_auth()`.
- **Unknown alert ID** returns `AlertNotFound` for both functions.
- **Direct update bypasses the flow.** `update_webhook` sets `webhook_hash` immediately and does **not** clear `pending_webhook_hash`. A pending proposal can therefore survive a direct update, and a later `confirm_webhook` will overwrite the directly-set hash. Use one path per rotation, not both.
- **No length check on propose.** `propose_webhook` stores the value as given. `update_webhook` requires exactly 64 characters. Validate the hash off-chain (see [Webhook Hash Scheme](#webhook-hash-scheme)) before proposing, because an invalid value is only caught when it is confirmed and used.

### Watcher behavior

Watchers should treat `webhook_hash` as the only live value. Ignore `pending_webhook_hash` for delivery. Switch delivery to the new hash when the `alert.webhook` event for that `config_id` is seen, or when a re-read of the alert shows a changed `webhook_hash`.

## Rule descriptor format

The `rules` field is a `Vec<String>` containing serialized rule descriptors. Each descriptor is a single string in the format `rule:<prefix>`, where `<prefix>` denotes the event or condition to watch for.

### Valid rule prefixes

| Prefix | Semantics |
|---|---|
| `rule:transfer` | Alert when the target contract emits a transfer-like action. |
| `rule:mint` | Alert when the target contract performs a mint or issuance event. |
| `rule:invoke` | Alert whenever the target contract is invoked (any function). |
| `rule:all` | Alert on any ledger entry change to the target contract (catch-all/debug). May produce very high webhook volume; watch rate limits. The watcher in stellar-txwatch-core must implement handlers for `rule:invoke` and `rule:all`. |

The alert registry stores these descriptors verbatim and validates that each entry uses a recognized prefix before accepting it. Off-chain watcher logic still interprets prefixes and applies the corresponding alert behavior.

---

## Functions

### `register_alert`

Registers a new alert configuration for a target contract address.

**Requires auth:** `owner`

**Validation:** Rule descriptors are checked against the known prefixes `rule:transfer`, `rule:mint`, `rule:invoke` and `rule:all`, and the contract panics if any rule is not recognized.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `owner` | `Address` | Owner of the alert config |
| `target_contract` | `Address` | Contract address to watch |
| `label` | `String` | Human-readable label |
| `webhook_hash` | `String` | SHA-256 hex digest of the webhook URL |
| `rules` | `Vec<String>` | Rule descriptors |

**Returns:** `u64` — the new config ID

---

### `update_alert`

Updates the rules and active status of an existing alert. Only the original owner may call this.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to update |
| `rules` | `Vec<String>` | New rule descriptors |
| `active` | `bool` | New active status |

**Returns:** nothing

**Errors:** Returns `ContractError::AlertNotFound` if ID does not exist; `ContractError::Unauthorized` if caller is not the owner.

---
### `initialize`

Initializes an optional admin for the contract. Can only be called once.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Address to assign as admin |

**Returns:** nothing

---

### `transfer_admin`

Transfers admin authority to a new address. Requires current admin auth.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin address |
| `new_admin` | `Address` | Address to become the new admin |

**Returns:** nothing

---

### `get_admin`

Returns the current admin address.

**Returns:** `Address`

---

### `set_per_owner_alert_limit`

Sets a global per-owner limit on active alerts. A value of `0` disables the limit.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin address |
| `limit` | `u32` | New per-owner active alert limit |

**Returns:** nothing

---

### `get_per_owner_alert_limit`

Returns the configured per-owner active alert limit, or `0` if no limit is set.

**Returns:** `u32`

---

### `remove_alert_by_admin`

Removes any alert config by ID. Requires admin auth.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin address |
| `config_id` | `u64` | ID of the alert to remove |

**Returns:** nothing

---
### `remove_alert`

Permanently removes an alert config. Only the original owner may call this.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to remove |

**Returns:** nothing

**Errors:** Returns `ContractError::AlertNotFound` if ID does not exist; `ContractError::Unauthorized` if caller is not the owner.

---

### `get_alert`

Retrieves a single alert config by ID.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `config_id` | `u64` | Alert config ID |

**Returns:** `Option<AlertConfig>` — `Some(config)` if found, `None` otherwise.

---

### `get_alert_active`

Cheap read-only function that returns just the `active` bool for a given alert ID, avoiding the cost of deserializing the full [`AlertConfig`](#alertconfig). The active flag is stored under a dedicated storage key (`DataKey::AlertActive`).

**Parameters**

| Name | Type | Description |
|---|---|---|
| `config_id` | `u64` | Alert config ID |

**Returns:** `Option<bool>` — `Some(true)` if active, `Some(false)` if inactive, `None` if the alert does not exist.

---

### `get_alerts_for_contract`

Returns all alert configs registered for a given target contract, including both active and inactive entries. Use [`get_active_alerts_for_contract`](#get_active_alerts_for_contract) to filter to active-only.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `target_contract` | `Address` | Contract address to query |

**Returns:** `Vec<AlertConfig>` — may be empty.

---

### `get_active_alerts_for_contract`

Returns only the active alert configs (`active == true`) registered for a given target contract. Inactive alerts are excluded.

This function is not gated by the `WatcherRegistry`, even when one is configured. See [Watcher Gating](./watcher-registry.md#watcher-gating) for the gated list.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `querier` | `Address` | Address performing the query (gated when a watcher registry is set) |
| `target_contract` | `Address` | Contract address to query |

**Returns:** `Vec<AlertConfig>` — may be empty.

---

### `get_active_alerts_paginated`

Paginated variant of `get_active_alerts_for_contract`. `offset`/`limit` apply to the active alerts after filtering; `limit` is clamped to `MAX_BATCH_SIZE`. (Named without `_for_contract` because Soroban function names are limited to 32 characters.)

| Name | Type | Description |
|---|---|---|
| `querier` | `Address` | Address performing the query (gated when a watcher registry is set) |
| `target_contract` | `Address` | Contract address to query |
| `offset` | `u32` | Number of active alerts to skip |
| `limit` | `u32` | Max alerts to return |

**Returns:** `Result<Vec<AlertConfig>, ContractError>` — `NotAWatcher` when gated and `querier` is not a watcher.

---

### `get_alert_ids_by_owner` / `get_alert_ids_for_contract`

Return only the live alert IDs (entries still in storage) from the owner / contract index, without deserializing configs. Cheaper than the full getters.

Both take a `querier` and are gated like the config getters: when a `WatcherRegistry` is configured, `querier` must be a registered watcher or the call returns `ContractError::NotAWatcher` (#453).

### `get_alerts_by_owner`

Returns all alert configs owned by a given address.

If a `WatcherRegistry` is configured, `querier` must be a registered watcher or the call returns `ContractError::NotAWatcher`.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `querier` | `Address` | Address performing the query |
| `owner` | `Address` | Owner address to query |

**Returns:** `Result<Vec<AlertConfig>, ContractError>` — `Ok(vec)` on success, may be empty.

---

### `update_label`

Updates only the label of an existing alert, leaving `rules` and `webhook_hash` unchanged. Use this to rename an alert without touching its configuration.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to update |
| `label` | `String` | New human-readable label (max 128 bytes) |

**Returns:** `Result<(), ContractError>`

**Errors:** `AlertNotFound` if ID does not exist; `Unauthorized` if caller is not the owner; `LabelTooLong` if `label` exceeds 128 bytes.

**Events:** `("alert", "update")` with `(id, caller, active)`.

---

### `update_webhook`

Updates the webhook hash for an existing alert. Use this to rotate webhook URLs without re-registering. Only the original owner may call this.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to update |
| `webhook_hash` | `String` | New hashed webhook URL |

**Returns:** `Result<(), ContractError>`

**Errors:** `AlertNotFound` if ID does not exist; `Unauthorized` if caller is not the owner.

---

### `propose_webhook`

Step 1 of the two-step webhook rotation flow. Stores the new hash in `pending_webhook_hash` without replacing the live `webhook_hash`. The old webhook remains active until the owner calls `confirm_webhook`, eliminating the window where the old webhook is deactivated before the new one is confirmed.

Calling `propose_webhook` again before confirming overwrites the previous pending hash, allowing the owner to correct a mistake.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to update |
| `new_webhook_hash` | `String` | SHA-256 hex digest of the new webhook URL |
| `note` | `Option<String>` | Optional reason for the rotation, max 128 bytes. Emitted in the event only; never stored on-chain (#420) |

**Returns:** `Result<(), ContractError>`

**Errors:** `AlertNotFound` if ID does not exist; `Unauthorized` if caller is not the owner; `LabelTooLong` if `note` exceeds 128 bytes.

**Events:** Emits `(Symbol("alert"), Symbol("wh_prop"))` with data `(id: u64, caller: Address, note: Option<String>)`.

---

### `confirm_webhook`

Step 2 of the two-step webhook rotation flow. Promotes `pending_webhook_hash` to `webhook_hash` and clears the pending field. Returns `NoPendingWebhook` if no rotation is in progress.

**Requires auth:** `caller` (must match `owner` of the config)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | ID of the alert to confirm rotation for |

**Returns:** `Result<(), ContractError>`

**Errors:** `AlertNotFound` if ID does not exist; `Unauthorized` if caller is not the owner; `NoPendingWebhook` if no pending hash exists.

**Events:** Emits `(Symbol("alert"), Symbol("wh_conf"))` with data `(id: u64, caller: Address)`.

---

### `renew_alert_ttl`

Extends the TTL of an alert and its index entries without modifying any data. This is the recommended way to keep an alert alive without changing `updated_at` (which would cause it to appear in incremental-sync results via `get_alerts_modified_since`).

**Requires auth:** `caller` (must be the `owner` of the config, or the delegate set by `set_delegate`)

**Parameters**

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | The alert owner or its delegate |
| `config_id` | `u64` | ID of the alert to renew |

**Returns:** `Result<(), ContractError>`

**Events:** emits `(alert, bump)` with data `(config_id, DEFAULT_TTL)`.

**Errors:** `AlertNotFound` if ID does not exist; `Unauthorized` if caller is neither the owner nor the delegate.

---

### `set_delegate` / `get_delegate`

Lets an alert owner authorise another address (a delegate, e.g. an off-chain watcher service) to call `renew_alert_ttl` for one alert (#425).

`set_delegate(caller, config_id, delegate: Option<Address>)`:

- **Requires auth:** `caller`, who must be the alert owner.
- Passing `Some(address)` sets the delegate; passing `None` clears it and revokes access immediately.
- The delegate cannot transfer, update or remove the alert, and cannot change the delegate.
- Removing the alert also clears its delegate.
- **Errors:** `AlertNotFound`; `Unauthorized` if caller is not the owner.

`get_delegate(config_id) -> Option<Address>` is a read-only query with no auth.

---

### `get_contract_alerts_paginated`

Returns a page of alert configs registered for a given target contract.

If a `WatcherRegistry` is configured, `querier` must be a registered watcher or the call returns `ContractError::NotAWatcher`.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `querier` | `Address` | Address performing the query |
| `target_contract` | `Address` | Contract address to query |
| `offset` | `u32` | Number of results to skip |
| `limit` | `u32` | Maximum number of results to return |

**Returns:** `Result<Vec<AlertConfig>, ContractError>` — may be empty.

---

### `get_alerts_by_owner_paginated`

Returns a page of alert configs owned by a given address.

If a `WatcherRegistry` is configured, `querier` must be a registered watcher or the call returns `ContractError::NotAWatcher`.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `querier` | `Address` | Address performing the query |
| `owner` | `Address` | Owner address to query |
| `offset` | `u32` | Number of results to skip |
| `limit` | `u32` | Maximum number of results to return |

**Returns:** `Result<Vec<AlertConfig>, ContractError>` — may be empty.

---

### `get_alert_count`

Returns the total number of alerts ever registered.

> **Monotonic counter:** This value only ever increases. Removing an alert (via `remove_alert` or `remove_alert_by_admin`) does not decrement the counter. It reflects the cumulative count of all registrations since contract initialization, not the number of currently active alerts.

**Parameters:** none

**Returns:** `u64`

---

### `set_watcher_registry`

Configures the `WatcherRegistry` contract address used for optional watcher-gating on read queries. Once set, `get_alerts_for_contract`, `get_alerts_by_owner`, and their paginated variants will cross-call `WatcherRegistry::is_watcher_authorized` before returning data. Admin only.

Setup steps, the gated and ungated function table, and the current limits on disabling gating are in [Watcher Gating](./watcher-registry.md#watcher-gating). Do not pass the zero address to disable gating: the code does not treat it as "off".

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin address |
| `watcher_registry` | `Address` | Address of the deployed `WatcherRegistry` contract |

**Returns:** nothing

---

### `get_watcher_registry`

Returns the configured `WatcherRegistry` contract address, or `None` if watcher-gating has not been enabled.

**Returns:** `Option<Address>`

---

## Errors

| Variant | Code | Description |
|---|---|---|
| `Unauthorized` | 1 | Caller is not the owner or admin |
| `AlertNotFound` | 2 | No alert exists for the given ID |
| `AlreadyInitialized` | 3 | `initialize` was called more than once |
| `NotInitialized` | 4 | Admin function called before `initialize` |
| `NoPendingWebhook` | 5 | `confirm_webhook` called but no rotation is in progress |

---

## Storage

- Alert configs are stored in **persistent storage** under `DataKey::Alert(id)`.
- Owner and contract indexes are stored in **persistent storage** under `DataKey::OwnerIndex` and `DataKey::ContractIndex`.
- The auto-incrementing ID counter is stored in **instance storage** under `NEXT_ID`.
- The optional admin address is stored in **instance storage** under `ADMIN`.
- The optional per-owner alert limit is stored in **instance storage** under `LIMIT`.
- The optional `WatcherRegistry` contract address is stored in **instance storage** under `WATCHREG`.

---

## Re-entrancy and cross-contract safety

This contract is safe to call from other Soroban contracts. Soroban executes contract calls atomically and does not allow classic callback-style re-entrancy into the same contract within the same transaction.

All state-mutating functions in `AlertRegistry` first enforce authorization with `require_auth()` and then perform local storage updates. There are no external callbacks or indirect contract calls during state mutation, so cross-contract invocation cannot introduce re-entrancy vulnerabilities.

## Compiled status of documented functions

This reference describes the deployed contract. These functions appear above but are **not** in
the compiled `lib.rs` (they exist only in the uncompiled `contract.rs`): `propose_webhook`,
`confirm_webhook` and `renew_alert_ttl`. Do not call them against a deployment built from `lib.rs`.

### `get_active_alert_count`

```text
get_active_alert_count(env, owner: Address) -> u32
```

Returns how many live alerts `owner` has. Read-only, no auth. Counts only the alerts in the owner's
index that still exist, so an archived alert is not counted.

`get_alert_count_for_contract` is not in the compiled contract and is not documented here.

### `add_admin` / `remove_admin` / `get_admins`
Multi-admin management. Any admin may add or remove another; removing the last admin fails with `LastAdmin`.
Events: `("admin", "add")`, `("admin", "remove")`. `transfer_admin` is deprecated (replaces the whole set).

### `get_alerts_updated_since`
`get_alerts_updated_since(target_contract, since, offset, limit) -> Vec<AlertConfig>` returns alerts with
`updated_at >= since`, paginated. **O(n)** in the size of the contract index (full scan and filter).

### `transfer_alert_ownership`
`transfer_alert_ownership(owner, config_id, new_owner)` moves an alert between owners, updating both owner
indexes and `updated_at`. Fails with `OwnerAlertLimitExceeded` if the new owner is at their limit.
Emits `("alert", "transfer")`.

### Tags: `register_alert(..., tags)` / `update_tags`
`AlertConfig.tags: Option<Vec<String>>` — at most 10 tags of at most 32 bytes each (`TooManyTags`, `TagTooLong`).
`update_tags(caller, config_id, tags)` replaces them; emits `("alert", "tags")`.
---

### `batch_register_alerts`

Registers up to `MAX_BATCH_SIZE` (50) alerts for `caller` in one call. Every input is validated and the per-owner limit is checked for the whole batch before any write, so the batch is atomic.

**Requires auth:** `caller`

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Owner of all registered alerts |
| `inputs` | `Vec<AlertInput>` | `{ target_contract, label, webhook_hash, rules }` per alert |

**Returns:** `Result<Vec<u64>, ContractError>` — IDs in insertion order.

**Errors:** `BatchTooLarge`, `InvalidWebhookHash`, `LabelTooLong`, `EmptyRules`, `TooManyRules`, `OwnerAlertLimitExceeded`.

**Events:** one `("alert", "register")` per alert.

---

### `pause_alert` / `unpause_alert`

Admin-only. Sets `active = false` / `true` on an alert without owner auth. Owners can still toggle `active` via `update_alert`.

**Requires auth:** `admin` (owner auth is not sufficient)

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Contract admin |
| `config_id` | `u64` | Alert to pause/unpause |

**Errors:** `Unauthorized` (non-admin), `AlertNotFound`.

**Events:** `("alert", "update")` with `(id, admin, active)`.

---

### `update_target_contract`

Moves an alert to a different target contract. The contract index is migrated from the old target to the new one, and `updated_at` is refreshed.

**Requires auth:** `caller` (must match `owner` of the config)

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Must be the alert owner |
| `config_id` | `u64` | Alert to update |
| `new_target` | `Address` | New target contract |

**Returns:** `Result<(), ContractError>`

**Errors:** `ContractPaused` while the contract is paused; `AlertNotFound` if the ID does not exist; `Unauthorized` if caller is not the owner.

**Events:** none emitted by this function.

---

### `deactivate_all_alerts`

Sets `active = false` on every alert owned by `caller` and returns how many were deactivated. Used as an emergency stop for one owner's alerts.

**Requires auth:** `caller`

| Name | Type | Description |
|---|---|---|
| `caller` | `Address` | Owner whose alerts are deactivated |

**Returns:** `u32` — the number of alerts that were deactivated.

**Errors:** the function panics (instead of returning an error) if the contract is paused.

**Events:** none emitted by this function.
