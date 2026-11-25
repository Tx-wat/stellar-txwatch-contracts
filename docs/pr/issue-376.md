# Use DEFAULT_TTL for all alert TTL extensions (#376)

## Summary

`extend_alert_ttl`, `extend_owner_index_ttl`, `extend_contract_index_ttl` and bulk renewal extended TTL with a hard-coded
`100, 100` ledgers (~8 minutes) while the rest of the contract uses `DEFAULT_TTL` (120,960 ledgers). Renewed alerts and
owner indexes therefore expired far sooner than documented.

## What changed

- **`contracts/alert-registry/src/storage.rs`** — replaced all four `extend_ttl(..., 100, 100)` calls with
  `extend_ttl(..., DEFAULT_TTL, DEFAULT_TTL)`.
- **`contracts/alert-registry/src/next_id_ttl_tests.rs`** — added a static regression test asserting no hard-coded
  `100, 100` extension remains and that `DEFAULT_TTL, DEFAULT_TTL` is used.

## Acceptance criteria addressed

- All `extend_ttl` calls use `DEFAULT_TTL`; `renew_alert_ttl` now refreshes for the documented seven days and the owner
  index no longer expires in minutes.

## Validation

`cargo check -p alert-registry` succeeds. The test target hits the same pre-existing `ed25519-dalek`/`rand_core`
dependency conflict noted in #375.

closes #376
