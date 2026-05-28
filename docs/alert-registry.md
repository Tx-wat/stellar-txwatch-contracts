# Alert Registry — Function Reference

Contract that stores alert configurations on-chain, keyed by contract address.

---

## Data Types

### `AlertConfig`

| Field | Type | Description |
|---|---|---|
| `label` | `String` | Human-readable name for the alert |
| `webhook_hash` | `String` | Hashed webhook URL (privacy-preserving) |
| `rules` | `Vec<String>` | Serialized rule descriptors |
| `owner` | `Address` | Address that owns this config |
| `target_contract` | `Address` | Contract being watched |
| `created_at` | `u64` | Ledger timestamp at creation |
| `updated_at` | `u64` | Ledger timestamp of last update |
| `active` | `bool` | Whether the alert is active |

---

## Rule descriptor format

The `rules` field is a `Vec<String>` containing serialized rule descriptors. Each descriptor is a single string in the format `rule:<prefix>`, where `<prefix>` denotes the event or condition to watch for.

### Valid rule prefixes

| Prefix | Semantics |
|---|---|
| `rule:transfer` | Alert when the target contract emits a transfer-like action. |
| `rule:mint` | Alert when the target contract performs a mint or issuance event. |

The alert registry stores these descriptors verbatim and does not validate or execute rule semantics on-chain. Off-chain watcher logic interprets prefixes and applies the corresponding alert behavior.

---

## Functions

### `register_alert`

Registers a new alert configuration for a target contract address.

**Requires auth:** `owner`

**Parameters**

| Name | Type | Description |
|---|---|---|
| `owner` | `Address` | Owner of the alert config |
| `target_contract` | `Address` | Contract address to watch |
| `label` | `String` | Human-readable label |
| `webhook_hash` | `String` | Hashed webhook URL |
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

**Panics:** `"alert not found"` if ID does not exist; `"unauthorized"` if caller is not the owner.

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

**Panics:** `"alert not found"` if ID does not exist; `"unauthorized"` if caller is not the owner.

---

### `get_alert`

Retrieves a single alert config by ID.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `config_id` | `u64` | Alert config ID |

**Returns:** `Option<AlertConfig>` — `Some(config)` if found, `None` otherwise.

---

### `get_alerts_for_contract`

Returns all alert configs registered for a given target contract.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `target_contract` | `Address` | Contract address to query |

**Returns:** `Vec<AlertConfig>` — may be empty.

---

### `get_alerts_by_owner`

Returns all alert configs owned by a given address.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `owner` | `Address` | Owner address to query |

**Returns:** `Vec<AlertConfig>` — may be empty.

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

**Returns:** nothing

**Panics:** `"alert not found"` if ID does not exist; `"unauthorized"` if caller is not the owner.

---

### `get_contract_alerts_paginated`

Returns a page of alert configs registered for a given target contract.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `target_contract` | `Address` | Contract address to query |
| `offset` | `u32` | Number of results to skip |
| `limit` | `u32` | Maximum number of results to return |

**Returns:** `Vec<AlertConfig>` — may be empty.

---

### `get_alerts_by_owner_paginated`

Returns a page of alert configs owned by a given address.

**Parameters**

| Name | Type | Description |
|---|---|---|
| `owner` | `Address` | Owner address to query |
| `offset` | `u32` | Number of results to skip |
| `limit` | `u32` | Maximum number of results to return |

**Returns:** `Vec<AlertConfig>` — may be empty.

---

### `get_alert_count`

Returns the total number of alerts ever registered (monotonic counter — does not decrease on removal).

**Parameters:** none

**Returns:** `u64`

---

## Storage

- Alert configs are stored in **persistent storage** under `DataKey::Alert(id)`.
- Owner and contract indexes are stored in **persistent storage** under `DataKey::OwnerIndex` and `DataKey::ContractIndex`.
- The auto-incrementing ID counter is stored in **instance storage**.
