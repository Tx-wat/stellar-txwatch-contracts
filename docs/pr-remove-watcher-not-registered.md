# test: remove_watcher on never-registered address

## Summary

Adds a test verifying that calling `remove_watcher` with an address that was never registered completes without error and leaves the watcher list unchanged.

## Changes

- `test_remove_watcher_not_registered` in `contracts/watcher-registry/src/lib.rs`
  - Calls `remove_watcher` on a fresh address (never registered)
  - Asserts the call returns `Ok(())`
  - Asserts the watcher list remains empty
  - Asserts `is_authorized` returns `false`

closes #59
