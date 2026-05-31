use soroban_sdk::{contracttype, contracterror, Address, String, Vec};

pub const MAX_BATCH_SIZE: u32 = 20;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 1,
    AlertNotFound = 2,
    AlreadyInitialized = 3,
    NotInitialized = 4,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

/// Storage key variants used to address persistent and instance entries.
#[contracttype]
pub enum DataKey {
    /// Stores an [`AlertConfig`] keyed by its numeric ID.
    Alert(u64),
    /// Stores the list of alert IDs owned by a given address.
    OwnerIndex(Address),
    /// Stores the list of alert IDs watching a given contract address.
    ContractIndex(Address),
    /// Monotonic counter used to generate unique alert IDs.
    NextId,
}

// ── Data types ────────────────────────────────────────────────────────────────

/// Input payload for a single alert in a `batch_register_alerts` call.
///
/// All fields mirror the per-alert parameters of `register_alert`; the owner
/// is taken from the `caller` argument of `batch_register_alerts` instead of
/// being stored here.
#[contracttype]
#[derive(Clone)]
pub struct AlertInput {
    /// Contract address to watch.
    pub target_contract: Address,
    /// Human-readable label (max 128 bytes).
    pub label: String,
    /// SHA-256 hex digest of the webhook URL.
    pub webhook_hash: String,
    /// Rule identifiers that trigger the alert.
    pub rules: Vec<String>,
}

/// On-chain configuration for a single alert.
///
/// Stored under [`DataKey::Alert`] with a TTL of 100 ledgers (~8 minutes).
/// See `docs/ttl.md` for expiry details and how to extend the TTL.
#[contracttype]
#[derive(Clone)]
pub struct AlertConfig {
    /// Human-readable label for the alert (max 128 bytes).
    pub label: String,
    /// SHA-256 hex digest of the webhook URL (the raw URL is never stored on-chain).
    pub webhook_hash: String,
    /// List of rule identifiers that trigger this alert (e.g. `"rule:transfer"`).
    pub rules: Vec<String>,
    /// Address that owns and may mutate this alert.
    pub owner: Address,
    /// Contract address being watched.
    pub target_contract: Address,
    /// Ledger timestamp at the time of registration.
    pub created_at: u64,
    /// Ledger timestamp of the most recent update.
    pub updated_at: u64,
    /// Whether the alert is currently active.
    pub active: bool,
}
