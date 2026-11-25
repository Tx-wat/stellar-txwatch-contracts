# ADR-002: Remove the dead `alert-registry/src/contract.rs`

Status: Accepted (#437)

## Context
`contracts/alert-registry/src/contract.rs` held a second, plain-Rust
implementation of `AlertRegistry` marked `#![allow(dead_code)]`. It was never
declared as a module in `lib.rs`, so the compiler never saw it. Its inline
tests referenced `AlertRegistryClient`, which is only generated from the
`#[contract]` / `#[contractimpl]` attributes in `lib.rs`, so the tests could
not run either. The file also duplicated `remove_alert_record` logic that
already lives in `lib.rs` and `storage.rs` (see #438).

## Decision
Delete `contract.rs`. The single contract entry point is `lib.rs`; shared
storage logic lives in `storage.rs`. Keeping a second, uncompiled copy only
created drift.

Option 2 from the issue (feature-gate it behind `testutils` as a helper) was
rejected: nothing needed the helper, and it would still be a second copy of
the contract logic to keep in sync.

## Consequences
- Dead code and its unrunnable tests are gone.
- The `bump_alert` doc link in `storage.rs` no longer points at the removed
  module.
