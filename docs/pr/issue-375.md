# Fix bulk renewal's target contract reference (#375)

## Summary

`storage::bulk_renew_one` read `cfg.target`, but `AlertConfig` stores the watched contract as `target_contract`. This
left the compiled source broken and prevented bulk TTL renewal from extending the contract index.

## What changed

- **`contracts/alert-registry/src/storage.rs`** — `bulk_renew_one` now passes `&cfg.target_contract` to
  `extend_contract_index_ttl`.
- **`contracts/alert-registry/src/next_id_ttl_tests.rs`** — added a regression test that repeatedly bulk-renews a live
  alert across half-TTL ledger advances and asserts the alert and its contract index remain readable.

## Validation

`cargo check -p alert-registry` succeeds. The test target cannot build in this environment due to a pre-existing
`ed25519-dalek`/`rand_core` version conflict in `soroban-env-host`, unrelated to this change.

closes #375
