#89 Refactor: Map already initialized panic to ContractError::AlreadyInitialized
Repo Avatar
Tx-wat/stellar-txwatch-contracts
Description\nPart of the typed error enum work. "already initialized" should map to ContractError::AlreadyInitialized.