#27 Feature: Add get_alerts_paginated for large indexes
Repo Avatar
Tx-wat/stellar-txwatch-contracts
Description\nget_alerts_by_owner and get_alerts_for_contract return all results at once. For owners with many alerts this can hit instruction limits. Add paginated variants with offset and limit parameters.
