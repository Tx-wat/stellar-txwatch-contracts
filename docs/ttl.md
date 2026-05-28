# TTL Values and Storage Expiry

## What is TTL in Soroban?

Soroban persistent storage entries have a **Time-To-Live (TTL)** measured in **ledgers**. Once an entry's TTL reaches zero, the Stellar network archives it and it becomes inaccessible until restored via a fee-paying restore operation.

## Current TTL Configuration

All persistent storage entries in `alert-registry` use a hardcoded TTL of **100 ledgers**:

```rust
env.storage().persistent().extend_ttl(&key, 100, 100);
//                                          ^^^  ^^^
//                                    min_ttl  max_ttl
```

This applies to:
- `DataKey::Alert(id)` — individual alert configs
- `DataKey::OwnerIndex(address)` — per-owner alert ID lists
- `DataKey::ContractIndex(address)` — per-contract alert ID lists

The `watcher-registry` uses **instance storage**, which has its own TTL managed by the network and is not explicitly extended in the current implementation.

## Wall-Clock Time Estimate

Stellar ledgers close approximately every **5 seconds**.

| Ledgers | Approximate wall-clock time |
|---------|-----------------------------|
| 100     | ~8 minutes 20 seconds       |
| 1,000   | ~1 hour 23 minutes          |
| 17,280  | ~24 hours                   |
| 120,960 | ~7 days                     |

**At the current setting of 100 ledgers, an alert entry expires roughly 8 minutes after the last write.**

## When Does the TTL Reset?

The TTL is extended on every mutating call that touches the entry:

| Function         | Entries extended |
|------------------|-----------------|
| `register_alert` | `Alert(id)`, `OwnerIndex`, `ContractIndex` |
| `update_alert`   | `Alert(id)` |
| `update_webhook` | `Alert(id)` |
| `remove_alert`   | Entry is deleted (no TTL needed) |

Read-only functions (`get_alert`, `get_alerts_for_contract`, `get_alerts_by_owner`) do **not** extend the TTL.

## Implications

- **Inactive alerts expire quickly.** An alert that is registered but never updated will be archived after ~8 minutes.
- **Watchers must keep alerts alive.** Any off-chain service relying on alert data should periodically call `update_alert` (or a dedicated bump function) to prevent expiry.
- **Archived entries are not deleted.** They can be restored via `RestoreFootprintOp`, but this requires paying a fee and is not currently automated.

## Recommendations

For production use, the TTL should be increased significantly. Common choices:

| Use case | Suggested TTL (ledgers) | Wall-clock |
|----------|------------------------|------------|
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
