# feat: add get_alerts_by_owner_paginated

## Summary

Implements paginated retrieval of alert configs by owner address to handle owners with large numbers of alerts without hitting Soroban instruction limits.

## Changes

- `get_alerts_by_owner_paginated(env, owner, offset, limit)` — returns a page of `AlertConfig` entries for the given owner using offset-based pagination.
- `get_contract_alerts_paginated(env, target_contract, offset, limit)` — same pagination support for contract-indexed lookups.
- Shared `configs_paginated` helper used by both functions.
- Fixed `update_webhook` and `remove_alert`: parameter was named `caller` but code referenced undefined `owner` variable.
- Removed duplicate `assert_owner` definition that caused a compile error.
- Added tests 18 and 19 covering pagination behaviour (first page, second page, out-of-bounds offset).

## Testing

Tests added in `contracts/alert-registry/src/lib.rs`:
- `test_get_alerts_by_owner_paginated` — verifies first page, second page, and empty result for out-of-bounds offset.
- `test_get_contract_alerts_paginated` — verifies mid-list page returns correct slice.

closes #39
