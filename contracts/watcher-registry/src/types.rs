use soroban_sdk::{contracterror, contracttype, Address, String};

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    NotInitialized = 3,
    /// Returned when trying to remove the last admin, which would lock the contract.
    LastAdmin = 4,
    /// Returned when the specified watcher is not currently registered.
    WatcherNotFound = 5,
    /// Returned by mutating entrypoints while the registry is paused.
    ContractPaused = 6,
    /// Returned when a batch exceeds `MAX_BATCH_SIZE`.
    BatchTooLarge = 7,
    /// Returned when registration would exceed `MAX_WATCHERS`.
    WatcherLimitExceeded = 8,
    /// Returned by `transfer_admin` when more than one admin is configured.
    MultipleAdminsExist = 9,
    /// Returned when a watcher label exceeds `MAX_LABEL_BYTES`.
    LabelTooLong = 10,
}

// ── Records ───────────────────────────────────────────────────────────────────

/// Metadata stored for every registered watcher.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatcherInfo {
    /// The watcher's address.
    pub address: Address,
    /// Optional human-readable label (empty when none was supplied).
    pub label: String,
    /// Ledger timestamp at which the watcher was registered.
    pub registered_at: u64,
    /// Admin that registered the watcher.
    pub registered_by: Address,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Storage key variants used to address instance and persistent entries.
#[contracttype]
pub enum DataKey {
    /// Stores the `Vec<Address>` of current admins.
    Admins,
    /// Stores the `Vec<Address>` of authorized watcher nodes.
    Watchers,
    /// Stores the running count of authorized watchers.
    WatcherCount,
    /// Stores the paused flag (circuit breaker).
    Paused,
    /// Stores a `Map<Address, u64>` of watcher expiry timestamps.
    Expiries,
    /// Stores the [`WatcherInfo`] record for a single watcher (persistent).
    WatcherInfo(Address),
}
