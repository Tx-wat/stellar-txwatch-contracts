use soroban_sdk::{contracttype, Address, String, Vec};

/// Maximum number of items accepted by any batched entrypoint, including
/// `batch_register_alerts` and `bulk_renew_ttl`.
pub const MAX_BATCH_SIZE: u32 = 20;


// ── Errors ────────────────────────────────────────────────────────────────────

// `ContractError` is defined once, in `lib.rs`, which is the enum the compiled
// contract actually returns. Re-export it here so there is a single source of truth.
pub use crate::ContractError;

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
    /// Stores the per-owner rate-limit state ([`RateLimitState`]).
    RateLimit(Address),
    /// Stores the contract-level pause flag (`bool`).
    Paused,
}

// ── Data types ────────────────────────────────────────────────────────────────

/// Per-owner rate-limit bookkeeping stored in instance storage.
///
/// Tracks how many alerts an owner has registered within the current window
/// and the ledger sequence at which that window started.
#[contracttype]
#[derive(Clone)]
pub struct RateLimitState {
    /// Ledger sequence at which the current window began.
    pub window_start: u32,
    /// Number of registrations recorded in the current window.
    pub count: u32,
}

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
/// Stored under [`DataKey::Alert`] with a default TTL of `DEFAULT_TTL` ledgers
/// (~24 hours). Use `AlertRegistry::bump_alert` to extend up to `MAX_TTL`.
/// See `docs/ttl.md` for expiry details.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// Pending webhook hash proposed via `propose_webhook` but not yet confirmed.
    ///
    /// `None` means no rotation is in progress. Once the owner calls
    /// `confirm_webhook`, this value is promoted to `webhook_hash` and cleared.
    /// This two-step flow prevents a window where the old webhook is deactivated
    /// before the new one is confirmed by the off-chain watcher.
    pub pending_webhook_hash: Option<String>,
}

/// Event payload emitted once per alert renewed by `bulk_renew_ttl`.
///
/// Mirrors the `alert.bump` event emitted by `bump_alert` so off-chain
/// indexers can consume both single and bulk renewals uniformly.
#[contracttype]
#[derive(Clone)]
pub struct BulkRenewEvent {
    /// ID of the alert whose TTL was extended.
    pub config_id: u64,
    /// Ledger sequence at which the renewal occurred.
    pub ledger: u32,
}
