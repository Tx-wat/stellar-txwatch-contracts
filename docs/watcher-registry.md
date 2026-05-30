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

**Requires auth:** `admin`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin |
| `watcher` | `Address` | Watcher address to authorize |

**Returns:** nothing

**Panics:** `"unauthorized"` if `admin` does not match the stored admin.

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

### `is_authorized`

Checks whether an address is a currently authorized watcher.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `watcher` | `Address` | Address to check |

**Returns:** `bool`

---

### `get_watchers`

Returns all currently authorized watcher addresses.

**Parameters:** none

**Returns:** `Vec<Address>` — may be empty.

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
