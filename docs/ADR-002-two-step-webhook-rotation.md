# ADR-002: Two-step webhook rotation (propose / confirm) instead of direct update

Status: Accepted

## Context
An alert's `webhook_hash` tells the off-chain watcher where to deliver notifications. Owners
sometimes need to rotate it, for example after a key or endpoint change.

A direct update (`update_webhook(caller, id, new_hash)`) replaces the active hash in one
transaction. If the new endpoint is wrong, misconfigured or unreachable, the alert is pointed at a
dead target immediately. Notifications are lost until the owner notices and rotates again, and
there is no point at which the old, known-good hash and the new hash are both in place.

The owner needs a way to stage the new hash, let the watcher verify it, and only then make it active.

## Decision
Rotation is split into two owner-authorised steps:

1. `propose_webhook(caller, id, new_hash)` stores the candidate in
   `AlertConfig.pending_webhook_hash`. The active `webhook_hash` is unchanged. Emits
   `("alert", "wh_prop")` with `(id, caller)`.
2. `confirm_webhook(caller, id)` moves `pending_webhook_hash` into `webhook_hash` and clears
   the pending field. It fails with `NoPendingWebhook` if nothing is pending. Emits
   `("alert", "webhook")` with `(id, caller)`.

Both steps require `caller.require_auth()` and the caller must be the alert owner. The old hash
stays active until confirmation.

`update_webhook` is kept for single-step changes where the owner accepts the risk, and is
documented in `docs/events.md` as `alert.webhook`.

## Consequences

**Positive**
- No window where the active target is missing or unverified. The old hash keeps receiving
  notifications until the new one is confirmed.
- Each step emits its own event, so indexers can show "rotation pending" and "rotation complete"
  separately.
- Proposing a new hash replaces any earlier pending hash, so an owner can correct a mistake before
  confirming.

**Negative**
- A rotation takes two transactions and two fees.
- While a rotation is pending, the alert holds two hashes in storage (`webhook_hash` and
  `pending_webhook_hash`). This adds one `String` per alert in the rotation window, and the entry
  is removed on confirm.
- Owners must remember to confirm. A pending hash that is never confirmed has no effect and
  should be surfaced by UIs as "rotation pending".
- Callers that want an atomic swap must still use `update_webhook`, which skips the safety step.

## Related
- Events: `docs/events.md` (`alert.wh_prop`, `alert.webhook`).
- Test coverage for the rotation flow: `docs/ISSUE_261_WEBHOOK_ROTATION_TESTS.md`.
