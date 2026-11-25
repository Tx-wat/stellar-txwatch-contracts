# Recovering archived alert storage

Soroban archives persistent entries whose TTL has run out. An archived alert is not deleted,
but reads no longer return it until it is restored. This page is the operator procedure for
finding archived entries and restoring them. The TTL rules are in [`ttl.md`](./ttl.md).

## 1. Detect archived entries

An entry is archived when its TTL has passed and no one has extended it. From an operator
machine:

1. Note the alert's config ID and the `AlertRegistry` contract ID.
2. Call `get_alert` with the config ID. A `None` result is either a removed alert or an
   archived one, so check the next step before concluding anything.
3. Read the entry's ledger key from Horizon or Soroban RPC. An archived entry is reported
   as archived, not as missing.

## 2. RestoreFootprintOp

`RestoreFootprintOp` is the Soroban operation that moves archived entries back into live state.
It has a fee, which depends on the size of the restored entries and the number of ledgers they
are extended for. Restoring costs more than a normal write, so do it only for alerts you need.

Restore before an alert is archived where you can: a keeper that bumps TTL (see section 5) avoids
the fee entirely.

## 3. Restore with the Stellar CLI

```sh
stellar contract restore \
  --id <ALERT_REGISTRY_CONTRACT_ID> \
  --key <ENTRY_KEY> \
  --network testnet \
  --source <OPERATOR_KEY_NAME>
```

Check `stellar contract restore --help` for the flags in your CLI version. Run the restore on
testnet first.

## 4. Restore with the JavaScript SDK

Build a transaction that contains a `RestoreFootprintOp` over the archived keys, simulate it to
get the resource fee, then sign and submit it:

1. Build the transaction with the keys in its footprint.
2. Simulate it with the RPC server. The simulation returns the restore fee.
3. Add the fee and sign with the operator key.
4. Submit, then poll until the transaction is confirmed.
5. Call `get_alert` again to confirm the alert is readable.

The exact builder calls depend on the SDK version. Follow the SDK's own examples for
`RestoreFootprintOp`.

## 5. Prevention

Archival is avoided, not repaired, by keeping TTL up:

- Run a keeper that calls `bump_alert` for alerts before their TTL expires.
- Renew long-lived alerts through `renew_alert_ttl`.
- Monitor expiry with the procedure in the keeper's own runbook, and alert on any alert within
  one day of expiry.

## 6. Failure mode: NextId expiry and ID collisions

Alert IDs come from a global counter (`NEXT_ID` in `contracts/alert-registry/src/lib.rs`).
Each registration reads the counter, uses that value as the new alert's ID, and increments it.
If the counter expires, the next registration reads the default `0`, so it can hand out an ID
that a live alert already uses.

Note: `contracts/alert-registry/src/storage.rs` has a second counter under `DataKey::NextId` in
persistent storage. Check which code path your deployed build uses before acting on what you see.

**Symptoms**

- `get_alert_count` returns a value lower than it did on an earlier read. The counter is
  monotonic, so a drop means the counter was reset.
- A `register_alert` call returns an ID that `get_alert` already returns with a different owner
  or target.
- Alerts that were visible earlier disappear, and `get_alert` returns `None` for IDs that
  should exist.

**Detection (read-only)**

1. Record `get_alert_count` from a monitoring job on a fixed schedule. Alert on any decrease.
2. Before any registration, compare the next expected ID with `get_alert` for that ID. If it
   returns an alert, the counter is behind.

**Recovery**

1. Stop registrations. Pause the operator tooling that calls `register_alert` or
   `batch_register_alerts`.
2. Do not register again to "fix" the counter. Every registration after a reset risks another collision.
3. Find the highest live ID. Use `get_alert` over the IDs in the owner and contract indexes, and
   record the highest one that is present.
4. Restore archived entries first (sections 1 to 4). An archived alert can look like a collision
   gap, so restore it before judging the counter.
5. Counter correction needs a contract change. The contract has no function that sets the
   counter directly. Plan an upgrade whose migration sets the counter above the highest live ID,
   and test it on testnet first.
6. Confirm afterwards that `get_alert_count` is at least the highest live ID and that a test
   registration returns a new ID.

**Prevention**

- Keep the counter alive: `bump_alert` and `renew_alert_ttl` extend it (see section 5).
- Keep a keeper running so the counter never reaches its expiry.

## 7. Failure mode: owner and contract index drift

Each alert is listed in two indexes: one per owner (`OwnerIndex`) and one per target contract
(`ContractIndex`). They are written when an alert is registered and removed when it is removed.
If an index entry and the alert entry get out of step, the indexes stop matching the alerts.

The read functions hide the most common form of drift. `get_alert_ids_by_owner` and
`get_alert_ids_for_contract` skip any ID whose alert entry is missing, so an orphan index entry
does not appear in their results.

**Symptoms**

- `get_alert_ids_by_owner` or `get_alert_ids_for_contract` returns fewer IDs than the owner
  or contract is known to have.
- An alert is returned by `get_alert` but is missing from the owner or contract listing.
- An alert's `owner` or `target_contract` does not match the index it appears under.

**Detection (read-only)**

1. For each owner you monitor, take the IDs from `get_alert_ids_by_owner` and call `get_alert`
   on each one. Any `None` means the index points at a missing alert, which is an orphan.
2. Check that each returned alert's `owner` equals the owner you queried. A mismatch means a
   misfiled index entry.
3. Repeat step 1 with `get_alert_ids_for_contract` for each target contract.
4. Run the same check on a sample of IDs from `get_alert_count`. An ID in range that is `None`
   and is not known to have been removed is a candidate for drift.

**Recovery**

1. Do not edit storage by hand. Indexes are only written by the contract's own functions.
2. Separate orphans from archived alerts. An ID that is `None` may be archived rather than
   removed. Check the ledger key (section 1) before calling it an orphan.
3. For a confirmed orphan, the normal path is to remove the ID with `remove_alert` (or
   `remove_alert_by_admin`) so the indexes are updated through the contract. If the removal
   fails, record the ID and escalate.
4. For a misfiled entry, re-register the alert under the correct owner and remove the bad one,
   so ownership changes go through the contract.
5. Re-run detection. Every ID should now be either a readable alert or a known removal.
6. If drift is widespread, stop writes and escalate. A contract upgrade with a migration may be
   needed. Do not attempt a wide manual repair.

**Prevention**

- Use the contract's own functions for every change. Do not write storage directly.
- Run the detection checks on a schedule, not only after an incident.

## 8. Checklist

- Confirm an entry is archived before restoring it.
- Test the restore on testnet first.
- Verify with `get_alert` afterwards.
- Keep a keeper running so restores are rare.
