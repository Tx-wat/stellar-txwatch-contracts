#117 Security: Add fixed-length validation for webhook_hash field
Repo Avatar
Tx-wat/stellar-txwatch-contracts
Description\nEnforce a fixed expected length on webhook_hash (e.g. 64 hex chars for SHA-256) to ensure only valid hashes are stored.