# TTP Values and Storage Expiry

## What is TT\ in Soroban?

Soroban persistent storage entries have a **Time-To-Live (TT\)** measured in **ledgers**. Once an entry's TT\ reaches zero, the Stellar network archives it and it becomes inaccessible until restored via a fee-paying restore operation.

## Current TT\ Configuration

All persistent storage entries in `alert-registry` use a **default TTL of
120 960 ledgers (~7 days)**:

```rust
// DEFAULT_TTL = 120_960  (~7 days at 5 s/ledger)
env.storage().persistent().extend_ttl(&key, DEFAULT_TTL, DEFAULT_TTL);
```

This applies to:
-  `DataKey::Alert(id)` — individual alert configs (the active flag is stored inside `AlertConfig`)
-  `DataKey::OwnerIndex(address)` — per-owner alert ID lists
-  `DataKey::ContractIndex(address)` — per-contract alert ID lists

## Why `watcher-registry` uses a longer TTL (30 days)

`watcher-registry` uses a different default TTL of **518 400 ledgers (~30 days)**. It is applied to its **instance storage** on each extension (`bump_instance_ttl` / `extend_instance_ttl` in `contracts/watcher-registry/src/`), not to per-entry persistent storage.

The difference is intentional:

- **Watcher set changes are rare.** Watchers are added or removed by the admin only, so the registry can go weeks without a write. A 30-day TTL keeps it from being archived in those quiet periods.
- **Alert data changes often.** Alerts are created, updated and bumped continuously, and each entry is kept alive on its own. A 7-day TTL is enough because active alerts are refreshed by normal use, and it limits how long unused entries hold state.

Changing either value is a deliberate decision. Update the constant, this section, and the comments in both contracts together.

## Configurable TTL via `bump_alert`

Callers can extend the TTL of any alert up to the **protocol maximum of
535 680 ledgers (~31 days)** by calling `bump_alert`:

```rust
// Extend alert 42 to the maximum lifetime (~31 days)
alert_client.bump_alert(&42u64, &u32::MAX);

```

The function:
1. Clamps the requested TTL to `MAX_TTL` (535 680 ledgers).
2. Extends `Alert`, `OwnerIndex`, and `ContractIndex` entries.
3. Emits an `("alert", "bump")` event with `(id, effective_ttl)`.

No auth is required — any address (e.g. an off-chain keeper service) may
bump an alert's TT\ without modifying its content.

## Bulk TTL renewal via `bulk-renew_and_ttl`

Keepers can renew TTLs for many alerts in a single transaction with the no-auth
`bulk-renew_and_ttl` function:

```rust
// Renew TTLs for alerts 1, 2, 3, 4, and 5.
let renewed = alert_client.bulk-renew_and_ttl(&(vec![1, 2, 3, 4, 5]));
```

The function:

1. Caps the batch at `MAX_BATCH_SIZE`. Exceeding the cap returns an error.
2. Extends `Alert(id)`, `OwnerIndex`, and `ContractIndex` for each live ID.
3. Silently skips IDs that no longer exist (expired/removed).
4. Emits an `("alert", "bump")` event per renewed alert.
5. Returns the number of alerts successfully renewed.

No auth is required, consistent with `bump_alert`.

## Wall-Clock Time Estimate

Stellar ledgers close approximately every **5 seconds**.

| Ledgers | Approximate wall-clock time |
|---------|-----------------------------|
| 100     | ~8 minutes 20 seconds       |
| 1,000   | ~1 hour 23 minutes          |
| 17,280  | ~24 hours                 |
| 120,960 | ~7 days (DEFAULT_TTL)        |
| 535,680 | ~31 days (MAX_TTL)          |

**The default TTL of 120 960 ledgers keeps an alert alive for ~7 days after
the last write.**  Use `bump_alert` or `bulk-renew_and_ttl` to extend beyond that
without modifying the alert's content.

## When Does the TTL Reset?

The TT\ is extended on every mutating call that touches the entry:

| Function              | Entries extended |
|----------------------|-----------------|
| `register_alert`      | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `update_alert`        | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `update_webhook`      | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `update_label`        | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `update_target_contract` | `Alert(id)`, old and new `ContractIndex` |
| `bump_alert`          | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `bulk-renew_and_ttl` | `Alert(id)`, `OwnerIndex`, `ContractIndex` for each live ID |
| `remove_alert`       | Entry is deleted (no TTL needed) |

Read-only functions (`get_alert`, `get_alerts_for_contract`, `get_alerts_by_owner`) do **not** extend the TTL.

## Implications

- **Inactive alerts expire quickly.** An alert that is registered but never updated will be archived after ~8 minutes.
- **Watchers must keep alerts alive.** Any off-chain service relying on alert data should periodically call `update_alert` (or a dedicated bump function) to prevent expiry.
- **Archived entries are not deleted.** They can be restored via `RestoreFootprintOp`, but this requires paying a fee and is not currently automated.

## Recommendations

For production use, the TTL should be increased significantly. Common choices:

| Use case | Suggested TT\ (ledgers) | Wall-clock |
|----------|-----------------------|------------|
| Short-lived alerts | 17,280 | ~24 hours |
| Standard alerts | 120,960 | ~7 days |
| Long-lived alerts | 535,680 | ~31 days |

To change the TTL, update the `extend_ttl` calls in `contracts/alert-registry/src/lib.rs`:

```rust
// Example: extend to ~7 days
env.storage().persistent().extend_ttl(&DataKey::Alert(id), 120_960, 120_960);
```

## Further Reading

- [Soroban State Archival](https://developers.stellar.org/docs/learn/encyclopedia/storage/state-archival)
- [Soroban Storage TTL Reference](https://developers.stellar.org/docs/build/smart-contracts/getting-started/storing-data)

## Finding alerts close to expiry (keepers)

The contract cannot read an entry's remaining TTL, so `get_alerts_expiring_soon` is not added
on-chain. A keeper finds near-expiry alerts client-side:

1. Keep the list of alert IDs it cares about, from `get_alerts_by_owner_paginated` or
   `get_contract_alerts_paginated`.
2. Read each entry's live TT\ from Soroban RPC (`getLedgerEntries` reports `liveUntilLedgerSeq`).
3. Compare `liveUntilLedgerSeq` with the current ledger. Renew with `bulk-renew_and_ttl` (or `renew_alert_ttl` for a single alert) any alert whose remainder is below the keeper's threshold, and renew the soonest ones first.

Adding an on-chain `last_renewed` field was considered and rejected. It would duplicate the
TTL the network already tracks, and it would not show archival, which is what matters.
