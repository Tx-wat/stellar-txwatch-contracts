# Watcher Registry — Function Reference

Contract that stores authorized watcher node addresses on-chain. Only registered watchers (trusted instances of `stellar-txwatch-core`) may interact with the alert registry.

---

## Functions

### `initialize`

Initializes the registry with an admin address. Can only be called once.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Initial admin of the registry |

**Returns:** nothing

**Panics:** `"already initialized"` if called more than once.

---

### `register_watcher`

Adds an address to the authorized watcher set. Idempotent — registering an already-registered watcher is a no-op.

The set is capped at `MAX_WATCHERS` (100). A registration that would exceed the cap returns `ContractError::WatcherLimitExceeded`.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin |
| `watcher` | `Address` | Watcher address to authorize |

**Returns:** nothing

**Errors:** `Unauthorized` if `admin` is not an admin; `MultipleAdminsExist` (6) if more than one admin exists.

**Decision (#236):** `transfer_admin` is only allowed when exactly one admin exists, so no admin is ever dropped silently. With several admins, rotate using `add_admin` + `remove_admin` (each emits an auditable event).

---

### `remove_watcher`

Removes an address from the authorized watcher set.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin |
| `watcher` | `Address` | Watcher address to remove |

**Returns:** nothing

**Panics:** `"unauthorized"` if `admin` does not match the stored admin.

---

### `is_watcher_authorized`

Checks whether an address is a currently authorized watcher.

Renamed from `is_authorized` for clarity in cross-contract call contexts — the name makes explicit *what* the address is being authorized as.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `watcher` | `Address` | Address to check |

**Returns:** `bool`

---

### `get_watchers` (deprecated)

Deprecated alias for `get_all_watchers`. It returns the same result and is kept for one release so existing callers can migrate. New code should call `get_all_watchers`.

**Parameters:** none

**Returns:** `Vec<Address>` — may be empty.

---

### `get_watcher_count`

Returns the number of registered watchers as a cheap integer read.

This function provides an efficient way to get the count of authorized watchers without requiring callers to fetch and count the full list.

**Parameters:** none

**Returns:** `u32` — the number of authorized watchers.

---

### `transfer_admin`

Transfers the admin role to a new address.

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin |
| `new_admin` | `Address` | Address to become the new admin |

**Returns:** nothing

**Panics:** `"unauthorized"` if `admin` does not match the stored admin.

---

### `is_admin`

Checks whether a given address is currently an admin of the registry.

Read-only, no authorization required.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `address` | `Address` | Address to check |

**Returns:** `bool` — `true` if the address is a registered admin, `false` otherwise (including if uninitialized).

---

### `get_admin`

Returns the current admin address.

**Parameters:** none

**Returns:** `Address`

**Panics:** `"not initialized"` if the contract has not been initialized.

---

## Storage

All state is stored in **instance storage**:

| Key | Value | Description |
|---|---|---|
| `"ADMIN"` | `Address` | Current admin address |
| `"WATCHERS"` | `Vec<Address>` | List of authorized watcher addresses |
---

## Re-entrancy and cross-contract safety

This contract is safe to call from other Soroban contracts. Soroban executes contract calls atomically and does not allow classic callback-style re-entrancy into the same contract within the same transaction.

All mutating entrypoints in `WatcherRegistry` require `admin.require_auth()` before updating storage, and no state-changing operation performs external callbacks. This makes the registry resistant to standard cross-contract re-entrancy attacks.

## Additional functions

### `add_admin`

```text
add_admin(env, caller: Address, new_admin: Address) -> Result<(), ContractError>
```

Adds `new_admin` to the admin set. `caller` must be an admin (`Unauthorized` otherwise). Adding an
existing admin changes nothing. Emits `("admin", "add")` with `(caller, new_admin)`.

### `remove_admin`

```text
remove_admin(env, caller: Address, target_admin: Address) -> Result<(), ContractError>
```

Removes `target_admin`. Refused with `NotAdmin` when `target_admin` is not in the admin set (#454), and with `LastAdmin` when it would leave the set empty. Emits
`("admin", "remove")`.

### `replace_watcher`

```text
replace_watcher(env, admin: Address, old_watcher: Address, new_watcher: Address) -> Result<(), ContractError>
```

Swaps one watcher for another in one call. `admin` must be an admin.

### `is_authorized`

```text
is_authorized(env, watcher: Address) -> bool
```

Returns `true` only for a registered watcher. The `AlertRegistry` gate calls this.

### `clear_all_watchers`

```text
clear_all_watchers(env, admin: Address) -> Result<(), ContractError>
```

Removes every watcher. Emits one `("watcher", "remove")` event per watcher.

### `get_admins`

```text
get_admins(env) -> Result<Vec<Address>, ContractError>
```

Returns the admin set. Read-only.

### `register_watcher` label and `WatcherInfo`

```text
register_watcher(env, admin, watcher, label: Option<String>) -> Result<(), ContractError>
```

`label` is optional (max 64 bytes, defaults to empty; longer returns `LabelTooLong` = 10). Each watcher gets a `WatcherInfo { address, label, registered_at, registered_by }` entry in persistent storage keyed by address. The `Watchers` vec stays as the enumeration index and `is_authorized` is an O(1) lookup.

### `get_watcher_registered_at`

```
get_watcher_registered_at(env, watcher: Address) -> Option<u64>
```

Returns the ledger timestamp at which `watcher` was registered (#426), read from `WatcherInfo.registered_at`. Returns `None` if the watcher is not registered (including after removal). Read-only, no auth.

### `get_watcher_info`

```text
get_watcher_info(env, watcher: Address) -> Option<WatcherInfo>
```

Read-only, no auth. Returns `None` if the watcher is not registered.

### `get_all_watchers`

```text
get_all_watchers(env) -> Vec<Address>
```

Read-only, no auth. Returns every registered watcher address.

### `get_all_watchers_paginated`

```text
get_all_watchers_paginated(env, offset: u32, limit: u32) -> Vec<Address>
```

Read-only, no auth. `limit` is capped at `MAX_PAGE_LIMIT` (50). Returns an empty vec when `offset` is past the end.

---

## Watcher Gating

Watcher gating is an optional check on the alert registry's read queries. When it is on,
a caller must be an authorized watcher in this registry before the alert registry returns
data. It is off by default. The alert-registry side is documented in
[`alert-registry.md`](./alert-registry.md#set_watcher_registry).

### Enable gating

1. Deploy `WatcherRegistry` and call `initialize(admin)`.
2. Register each trusted watcher node with `register_watcher(admin, watcher)`.
3. Deploy `AlertRegistry` and call `initialize(admin)` on it.
4. As the alert-registry admin, call `set_watcher_registry(admin, <WATCHER_REGISTRY_ID>)`
   on the alert registry.
5. Confirm with `get_watcher_registry()` on the alert registry. It should return the
   watcher-registry address.
6. Test with a registered watcher and an unregistered address. The first call should
   succeed and the second should fail with `ContractError::NotAWatcher`.

Each gated read makes a cross-contract call to `is_watcher_authorized`, so a watcher whose
registration has expired (see `renew_watcher`) is rejected the same way as an unregistered one.

### Gated vs ungated functions (alert registry)

| Function | Gated | Notes |
|---|---|---|
| `get_alerts_for_contract` | Yes | `querier` must be a registered watcher |
| `get_contract_alerts_paginated` | Yes | `querier` must be a registered watcher |
| `get_alerts_by_owner` | Yes | `querier` must be a registered watcher |
| `get_alerts_by_owner_paginated` | Yes | `querier` must be a registered watcher |
| `get_alert` | No | Single alert by ID |
| `get_alert_active` | No | Active flag by ID |
| `get_alert_ids_for_contract` | No | Returns IDs only |
| `get_alert_ids_by_owner` | No | Returns IDs only |
| `get_active_alerts_for_contract` | No | Not gated in the current code |
| `get_active_alerts_paginated` | No | Not gated in the current code |
| `get_alerts_modified_since` / `get_alerts_updated_since` | No | |
| `get_alert_count` / `get_active_alert_count` | No | Counts only |

Only the four functions marked **Yes** check the watcher registry.

### Disabling gating

The alert registry's `set_watcher_registry` doc comment says that passing the zero address
disables gating. The current code does not implement this: it stores whatever address it is
given and does not special-case the zero address. Calling a gated function after that would
make a cross-contract call to an address that is not a watcher registry, and the call would fail.

Until that is fixed, there is no supported way to turn gating off once it is on. Do not rely on the
zero-address approach. Each call can only replace the configured address with another one. It cannot clear it.
