use soroban_sdk::{contracttype, contracterror, Address, String, Vec};

/// Maximum number of items accepted by any batched entrypoint, including
/// `batch_register_alerts` and `bulk_renew_ttl`.
pub const MAX_BATCH_SIZE: u32 = 20;

/// Maximum number of alert registrations allowed per owner within a single
/// rate-limit window. See [`RATE_LIMIT_WINDOW`].
pub const RATE_LIMIT_COUNT: u32 = 10;

/// Length of the rate-limit window, measured in ledgers (~8 minutes at
/// ~5s per ledger). An owner may register at most [`RATE_LIMIT_COUNT`] alerts
/// per window.
pub const RATE_LIMIT_WINDOW: u32 = 100;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 1,
    AlertNotFound = 2,
    AlreadyInitialized = 3,
    NotInitialized = 4,
    /// Returned when `confirm_webhook` is called but no rotation is pending.
    NoPendingWebhook = 5,    LabelTooLong = 6,
    TooManyRules = 7,
    InvalidRuleDescriptor = 8,
    OwnerAlertLimitExceeded = 9,
    DuplicateAlertId = 10,
    /// Returned when an owner exceeds [`RATE_LIMIT_COUNT`] registrations
    /// within [`RATE_LIMIT_WINDOW`] ledgers.
    RateLimitExceeded = 11,
    /// Returned when the alert ID counter would overflow `u64`.
    ///
    /// The counter starts at 0 and increments by one per registration. In
    /// release builds integer overflow wraps silently, so at `u64::MAX` the
    /// next increment would wrap to 0 and overwrite alert ID 0. `next_id`
    /// uses `checked_add` and returns this error instead of wrapping.
    CounterOverflow = 14,
    /// Returned when a mutating function is called while the contract is paused.
    ContractPaused = 15,
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
