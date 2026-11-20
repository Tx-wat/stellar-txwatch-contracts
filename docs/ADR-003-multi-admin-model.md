# ADR-003: Multi-admin model (N-of-N independent admins instead of M-of-N multisig)

Status: Accepted

## Context
Both contracts (`alert-registry` and `watcher-registry`) originally stored a single admin address.
A single key controlled pausing, per-owner limits, watcher-registry linking and admin rotation. That
is a single point of failure, and losing the key locks the registry.

Both contracts now store a set of admins, `Vec<Address>`, under the instance key `ADMIN`.
The open question was how many admins must agree before an admin action runs.

## Decision
Use an **N-of-N independent admin set**: every address in the set is a full admin, and any single
admin may perform any admin action on its own.

- `add_admin(caller, new_admin)` and `remove_admin(caller, target)` may be called by any existing
  admin. Each emits an auditable `("admin", "add")` or `("admin", "remove")` event.
- `remove_admin` refuses to remove the last admin (`LastAdmin`), so the set is never empty.
- `add_admin` refuses duplicates (`AlreadyAdmin`). `remove_admin` on a non-member returns `NotAdmin`.
- `watcher-registry` only allows `transfer_admin` when exactly one admin exists
  (`MultipleAdminsExist` otherwise). Rotation with several admins uses `add_admin` followed by
  `remove_admin`, so no admin is ever dropped silently.

Why not M-of-N multisig: a threshold would require every admin action to be collected from several
signers, in one transaction (Soroban auth entries) or across several transactions with off-chain
coordination. That adds operational cost to routine actions such as pausing the registry during an
incident, and the contract would need proposal storage and expiry logic. An N-of-N set lets a pause or limit
change run with one signature, which matters during an incident.

## Consequences

**Positive**
- Simple to operate. An incident response needs one reachable admin, not a quorum.
- Admin changes are visible on-chain as individual events. Indexers can alert on every `admin.add`.
- Adding a backup admin protects against losing one key, without changing the approval rules.

**Negative / risk**
- **Higher risk if one admin key is compromised.** A stolen admin key can add its own admins, remove
  the others, pause the registry and change limits. The model has no veto.
- Mitigations: keep admin keys in separate custody, monitor `admin.add` and `admin.remove`
  events, and keep the admin set small.
- Removing a compromised admin is possible only while another admin is still in control. Rotate
  before the last honest admin is lost.

## Migration from the single-admin format
The `ADMIN` instance entry used to hold a single `Address`. `load_admins` reads either format:
it tries `Vec<Address>` first and falls back to a bare `Address`, returning `vec![admin]`. The next
write (`set_admins`) stores the vector. No separate migration transaction is needed, and existing
deployments keep working unchanged.

Upgrade checklist:
1. Deploy the new WASM. Existing admin addresses are read as a one-element set.
2. Check `get_admins()` returns the expected single admin.
3. Optionally call `add_admin` for each backup admin. Each call writes the vector format.

## Related
- Events: `("admin", "add")` and `("admin", "remove")` are emitted by `alert-registry`
  (`add_admin`, `remove_admin`). `docs/events.md` does not list them yet.
- Admin-transfer event: `docs/events.md` (`admin.transfer`).
- Watcher-registry function reference: `docs/watcher-registry.md`.
- Storage layout decisions: `docs/ADR-001-watcher-authorization-storage.md`.
