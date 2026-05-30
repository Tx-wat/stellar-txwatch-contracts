#![no_std]
#![warn(clippy::pedantic)]
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, contracterror, symbol_short, vec, Address, Env, String, Vec,
};

// ── Storage keys ────────────────────────────────────────────────────────────

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

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 1,
    AlertNotFound = 2,
    AlreadyInitialized = 3,
    NotInitialized = 4,
    /// Returned when a watcher registry is configured and the querying address
    /// is not a registered watcher.
    NotAWatcher = 5,
}

// ── Data types ───────────────────────────────────────────────────────────────

/// On-chain configuration for a single alert.
///
/// Stored under [`DataKey::Alert`] with a TTL of 100 ledgers (~8 minutes).
/// See `docs/ttl.md` for expiry details and how to extend the TTL.
#[contracttype]
#[derive(Clone)]
pub struct AlertConfig {
    /// Human-readable label for the alert.
    pub label: String,
    /// SHA-256 hash of the webhook URL (the raw URL is never stored on-chain).
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

// ── Contract ─────────────────────────────────────────────────────────────────

/// On-chain registry for alert configurations.
///
/// Each alert is keyed by a monotonically increasing `u64` ID and indexed by
/// both owner address and target contract address for efficient lookups.
///
/// # Watcher-gating (optional)
/// When a `WatcherRegistry` contract address is configured via
/// [`set_watcher_registry`], the read-only query functions
/// (`get_alerts_for_contract`, `get_alerts_by_owner`, and their paginated
/// variants) will perform a cross-contract call to verify that the querying
/// address is a registered watcher before returning data. Callers that are not
/// registered watchers receive [`ContractError::NotAWatcher`].
///
/// If no watcher registry is configured the gating is skipped and the
/// functions behave as before.
///
/// # Storage and TTL
/// All persistent entries are extended by 100 ledgers (~8 minutes) on every
/// write. See `docs/ttl.md` for implications and how to tune this value.
#[contract]
pub struct AlertRegistry;

// ── Cross-contract interface for WatcherRegistry ─────────────────────────────

/// Minimal client interface for calling `WatcherRegistry::is_watcher_authorized`
/// from within `AlertRegistry`.
mod watcher_registry_interface {
    use soroban_sdk::{contractclient, Address, Env};

    #[contractclient(name = "WatcherRegistryClient")]
    pub trait WatcherRegistry {
        fn is_watcher_authorized(env: Env, watcher: Address) -> bool;
    }
}

use watcher_registry_interface::WatcherRegistryClient as ExtWatcherClient;

#[contractimpl]
impl AlertRegistry {
    // ── Admin / configuration ─────────────────────────────────────────────

    /// Initialize the optional admin role for the registry. Can only be called once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&symbol_short!("ADMIN")) {
            return Err(ContractError::AlreadyInitialized);
        }
        env.storage().instance().set(&symbol_short!("ADMIN"), &admin);
        Ok(())
    }

    /// Transfer the admin role to a new address (admin only).
    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;
        env.storage().instance().set(&symbol_short!("ADMIN"), &new_admin);
        Ok(())
    }

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .unwrap_or_else(|| panic_with_error!(&env, ContractError::NotInitialized))
    }

    /// Set a per-owner active alert limit (admin only). A value of `0` means no limit.
    pub fn set_per_owner_alert_limit(
        env: Env,
        admin: Address,
        limit: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;
        env.storage().instance().set(&symbol_short!("LIMIT"), &limit);
        Ok(())
    }

    /// Get the configured per-owner active alert limit, or `0` if none is set.
    pub fn get_per_owner_alert_limit(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("LIMIT"))
            .unwrap_or(0u32)
    }

    /// Configure the `WatcherRegistry` contract address used for optional
    /// watcher-gating on read queries (admin only).
    ///
    /// Once set, `get_alerts_for_contract`, `get_alerts_by_owner`, and their
    /// paginated variants will cross-call `WatcherRegistry::is_watcher_authorized`
    /// before returning data. Pass the zero address to disable gating.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `admin`.
    pub fn set_watcher_registry(
        env: Env,
        admin: Address,
        watcher_registry: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;
        env.storage()
            .instance()
            .set(&symbol_short!("WATCHREG"), &watcher_registry);
        Ok(())
    }

    /// Return the configured `WatcherRegistry` contract address, or `None` if
    /// watcher-gating has not been enabled.
    pub fn get_watcher_registry(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("WATCHREG"))
    }

    // ── Alert mutations ───────────────────────────────────────────────────

    /// Register a new alert config and return its assigned ID.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `owner`.
    ///
    /// # Arguments
    /// * `owner` - Address that will own and control this alert.
    /// * `target_contract` - Contract address to watch.
    /// * `label` - Human-readable name for the alert.
    /// * `webhook_hash` - SHA-256 hash of the destination webhook URL.
    /// * `rules` - Rule identifiers that should trigger the alert.
    ///
    /// # Returns
    /// The new alert's numeric ID.
    pub fn register_alert(
        env: Env,
        owner: Address,
        target_contract: Address,
        label: String,
        webhook_hash: String,
        rules: Vec<String>,
    ) -> u64 {
        owner.require_auth();

        if label.len() > 128 {
            panic!("label exceeds 128 bytes");
        }

        if rules.len() > 50 {
            panic!("too many rules: maximum is 50");
        }

        Self::assert_per_owner_limit(&env, &owner);
        Self::validate_rules(&env, &rules);

        let id = Self::next_id(&env);
        let now = env.ledger().timestamp();

        let config = AlertConfig {
            label,
            webhook_hash,
            rules,
            owner: owner.clone(),
            target_contract: target_contract.clone(),
            created_at: now,
            updated_at: now,
            active: true,
        };

        env.storage().persistent().set(&DataKey::Alert(id), &config);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Alert(id), 100, 100);
        Self::push_owner_index(&env, &owner, id);
        Self::push_contract_index(&env, &target_contract, id);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("register")),
            (id, owner, target_contract),
        );

        id
    }

    /// Update the rules and active flag of an existing alert.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    pub fn update_alert(
        env: Env,
        caller: Address,
        config_id: u64,
        rules: Vec<String>,
        active: bool,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::assert_owner(&config, &caller)?;
        Self::validate_rules(&env, &rules);

        config.rules = rules;
        config.active = active;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Alert(config_id), 100, 100);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerIndex(config.owner.clone()), 100, 100);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::ContractIndex(config.target_contract.clone()), 100, 100);
        Ok(())
    }

    /// Update the webhook hash for an existing alert.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    pub fn update_webhook(
        env: Env,
        caller: Address,
        config_id: u64,
        webhook_hash: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::assert_owner(&config, &caller)?;

        config.webhook_hash = webhook_hash;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Alert(config_id), 100, 100);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerIndex(config.owner.clone()), 100, 100);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::ContractIndex(config.target_contract.clone()), 100, 100);
        Ok(())
    }

    /// Update only the label of an existing alert, leaving rules and webhook hash unchanged.
    ///
    /// Use this when you want to rename an alert without touching its rules or
    /// rotating its webhook URL.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Errors
    /// Returns [`ContractError::AlertNotFound`] if `config_id` does not exist.
    /// Returns [`ContractError::Unauthorized`] if `caller` is not the alert owner.
    ///
    /// # Panics
    /// Panics if `label` exceeds 128 bytes.
    pub fn update_label(
        env: Env,
        caller: Address,
        config_id: u64,
        label: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        if label.len() > 128 {
            panic!("label exceeds 128 bytes");
        }

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::assert_owner(&config, &caller)?;

        config.label = label;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage().persistent().extend_ttl(&DataKey::Alert(config_id), 100, 100);
        env.storage().persistent().extend_ttl(&DataKey::OwnerIndex(config.owner.clone()), 100, 100);
        env.storage().persistent().extend_ttl(&DataKey::ContractIndex(config.target_contract.clone()), 100, 100);
        Ok(())
    }

    /// Remove an alert config from storage.
    ///
    /// Also removes the alert ID from the owner and contract indexes.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    pub fn remove_alert(
        env: Env,
        caller: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::assert_owner(&config, &caller)?;
    pub fn get_alerts_for_contract(
        env: Env,
        querier: Address,
        target_contract: Address,
    ) -> Result<Vec<AlertConfig>, ContractError> {
        Self::assert_watcher_if_configured(&env, &querier)?;
        let ids = Self::contract_index(&env, &target_contract);
        Ok(Self::configs_for_ids(&env, &ids))
    }

    /// Retrieve only the active alert configs that watch a given contract address.
    ///
    /// Equivalent to [`get_alerts_for_contract`] but filters out any entries
    /// where `active == false`. Returns an empty vec if no active alerts exist
    /// for `target_contract`.
    pub fn get_active_alerts_for_contract(env: Env, target_contract: Address) -> Vec<AlertConfig> {
        let ids = Self::contract_index(&env, &target_contract);
        Self::active_configs_for_ids(&env, &ids)
    }

    /// Retrieve all alert configs owned by a given address.
    ///
    /// If a `WatcherRegistry` is configured, `querier` must be a registered
    /// watcher or the call returns [`ContractError::NotAWatcher`].
    ///
    /// Returns an empty vec if `owner` has no registered alerts.
    pub fn get_alerts_by_owner(
        env: Env,
        querier: Address,
        owner: Address,
    ) -> Result<Vec<AlertConfig>, ContractError> {
        Self::assert_watcher_if_configured(&env, &querier)?;
        let ids = Self::owner_index(&env, &owner);
        Ok(Self::configs_for_ids(&env, &ids))
    }

    /// Get a page of alert configs for a target contract (offset + limit).
    ///
    /// If a `WatcherRegistry` is configured, `querier` must be a registered
    /// watcher or the call returns [`ContractError::NotAWatcher`].
    pub fn get_contract_alerts_paginated(
        env: Env,
        querier: Address,
        target_contract: Address,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<AlertConfig>, ContractError> {
        Self::assert_watcher_if_configured(&env, &querier)?;
        let ids = Self::contract_index(&env, &target_contract);
        Ok(Self::configs_paginated(&env, &ids, offset, limit))
    }

    /// Get a page of alert configs owned by an address (offset + limit).
    ///
    /// If a `WatcherRegistry` is configured, `querier` must be a registered
    /// watcher or the call returns [`ContractError::NotAWatcher`].
    pub fn get_alerts_by_owner_paginated(
        env: Env,
        querier: Address,
        owner: Address,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<AlertConfig>, ContractError> {
        Self::assert_watcher_if_configured(&env, &querier)?;
        let ids = Self::owner_index(&env, &owner);
        Ok(Self::configs_paginated(&env, &ids, offset, limit))
    }

    /// Get the total number of alerts ever registered.
    ///
    /// This is a **monotonic counter** — it only increases and is never
    /// decremented when alerts are removed. Use [`get_active_alert_count`]
    /// if you need the number of currently live alerts for a given owner.
    #[must_use]
    pub fn get_alert_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&symbol_short!("NEXT_ID"))
            .unwrap_or(0u64)
    }

    /// Get the number of currently active (non-removed) alerts owned by `owner`.
    ///
    /// Unlike [`get_alert_count`], this reflects removals and only counts
    /// alerts whose storage entries are still live.
    pub fn get_active_alert_count(env: Env, owner: Address) -> u32 {
        let ids = Self::owner_index(&env, &owner);
        let mut count: u32 = 0;
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if env.storage().persistent().has(&DataKey::Alert(id)) {
                count += 1;
            }
        }
        count
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// If a `WatcherRegistry` contract address is stored in instance storage,
    /// perform a cross-contract call to verify that `querier` is a registered
    /// watcher. Returns `Ok(())` when no registry is configured (gating is
    /// disabled) or when the querier passes the check.
    fn assert_watcher_if_configured(env: &Env, querier: &Address) -> Result<(), ContractError> {
        let maybe_registry: Option<Address> = env
            .storage()
            .instance()
            .get(&symbol_short!("WATCHREG"));

        if let Some(registry_addr) = maybe_registry {
            let client = ExtWatcherClient::new(env, &registry_addr);
            if !client.is_watcher_authorized(querier) {
                return Err(ContractError::NotAWatcher);
            }
        }
        Ok(())
    }

    fn assert_owner(config: &AlertConfig, caller: &Address) -> Result<(), ContractError> {
        if config.owner == *caller {
            Ok(())
        } else {
            Err(ContractError::Unauthorized)
        }
    }

    fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        if !env.storage().instance().has(&symbol_short!("ADMIN")) {
            return Err(ContractError::NotInitialized);
        }
        let admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .unwrap();
        if admin == *caller {
            Ok(())
        } else {
            Err(ContractError::Unauthorized)
        }
    }

    fn assert_per_owner_limit(env: &Env, owner: &Address) {
        let limit = Self::get_per_owner_alert_limit(env.clone());
        if limit > 0 && Self::get_active_alert_count(env.clone(), owner.clone()) >= limit {
            panic!("owner alert limit exceeded");
        }
    }

    fn validate_rules(env: &Env, rules: &Vec<String>) {
        if rules.len() > 50 {
            panic!("too many rules: maximum is 50");
        }
        for i in 0..rules.len() {
            Self::validate_rule(env, &rules.get(i).unwrap());
        }
    }

    fn validate_rule(env: &Env, rule: &String) {
        let transfer = String::from_str(env, "rule:transfer");
        let mint = String::from_str(env, "rule:mint");
        if *rule != transfer && *rule != mint {
            panic!("invalid rule descriptor");
        }
    }

    fn remove_alert_record(env: &Env, config: &AlertConfig, config_id: u64, caller: &Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::Alert(config_id));

        Self::remove_from_owner_index(env, &config.owner, config_id);
        Self::remove_from_contract_index(env, &config.target_contract, config_id);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("remove")),
            (config_id, caller.clone()),
        );
    }

    /// Atomically read and increment the global alert ID counter.
    ///
    /// Returns the current value before incrementing, so the first ID is `0`.
    fn next_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("NEXT_ID"))
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&symbol_short!("NEXT_ID"), &(id + 1));
        id
    }

    /// Load the list of alert IDs owned by `owner`, or an empty vec.
    fn owner_index(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerIndex(owner.clone()))
            .unwrap_or_else(|| vec![env])
    }

    /// Load the list of alert IDs watching `target`, or an empty vec.
    fn contract_index(env: &Env, target: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ContractIndex(target.clone()))
            .unwrap_or_else(|| vec![env])
    }

    /// Append `id` to the owner's index and persist it with a refreshed TTL.
    fn push_owner_index(env: &Env, owner: &Address, id: u64) {
        let mut ids = Self::owner_index(env, owner);
        for i in 0..ids.len() {
            if ids.get(i).unwrap() == id {
                panic!("duplicate alert id in owner index");
            }
        }
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerIndex(owner.clone()), &ids);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerIndex(owner.clone()), 100, 100);
    }

    /// Append `id` to the contract's index and persist it with a refreshed TTL.
    fn push_contract_index(env: &Env, target: &Address, id: u64) {
        let mut ids = Self::contract_index(env, target);
        for i in 0..ids.len() {
            if ids.get(i).unwrap() == id {
                panic!("duplicate alert id in contract index");
            }
        }
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::ContractIndex(target.clone()), &ids);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::ContractIndex(target.clone()), 100, 100);
    }

    /// Remove `id` from the owner's index and persist the updated list.
    fn remove_from_owner_index(env: &Env, owner: &Address, id: u64) {
        let ids = Self::owner_index(env, owner);
        let mut updated: Vec<u64> = vec![env];
        for i in 0..ids.len() {
            let v = ids.get(i).unwrap();
            if v != id {
                updated.push_back(v);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::OwnerIndex(owner.clone()), &updated);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerIndex(owner.clone()), 100, 100);
    }

    /// Remove `id` from the contract's index and persist the updated list.
    fn remove_from_contract_index(env: &Env, target: &Address, id: u64) {
        let ids = Self::contract_index(env, target);
        let mut updated: Vec<u64> = vec![env];
        for i in 0..ids.len() {
            let v = ids.get(i).unwrap();
            if v != id {
                updated.push_back(v);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::ContractIndex(target.clone()), &updated);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::ContractIndex(target.clone()), 100, 100);
    }

    /// Resolve a list of alert IDs to their stored [`AlertConfig`] values.
    ///
    /// IDs that no longer exist in storage (expired or removed) are silently
    /// skipped.
    fn configs_for_ids(env: &Env, ids: &Vec<u64>) -> Vec<AlertConfig> {
        let mut out: Vec<AlertConfig> = vec![env];
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if let Some(cfg) = env.storage().persistent().get(&DataKey::Alert(id)) {
                out.push_back(cfg);
            }
        }
        out
    }

    /// Like [`configs_for_ids`] but only includes entries where `active == true`.
    ///
    /// IDs that no longer exist in storage are silently skipped, as are configs
    /// whose `active` field is `false`.
    fn active_configs_for_ids(env: &Env, ids: &Vec<u64>) -> Vec<AlertConfig> {
        let mut out: Vec<AlertConfig> = vec![env];
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if let Some(cfg) = env.storage().persistent().get::<DataKey, AlertConfig>(&DataKey::Alert(id)) {
                if cfg.active {
                    out.push_back(cfg);
                }
            }
        }
        out
    }

    fn configs_paginated(
        env: &Env,
        ids: &Vec<u64>,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let mut out: Vec<AlertConfig> = vec![env];
        let len = ids.len();
        let start = offset.min(len);
        let end = (offset + limit).min(len);
        for i in start..end {
            let id = ids.get(i).unwrap();
            if let Some(cfg) = env.storage().persistent().get(&DataKey::Alert(id)) {
                out.push_back(cfg);
            }
        }
        out
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env, String};

    fn setup() -> (Env, AlertRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AlertRegistry);
        let client = AlertRegistryClient::new(&env, &contract_id);
        (env, client)
    }

    fn str(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // ── Helpers shared by watcher-gating tests ────────────────────────────

    #[cfg(feature = "testutils")]
    fn setup_with_watcher_registry() -> (
        Env,
        AlertRegistryClient<'static>,
        watcher_registry::WatcherRegistryClient<'static>,
    ) {
        use watcher_registry::WatcherRegistry;
        let env = Env::default();
        env.mock_all_auths();

        let alert_id = env.register_contract(None, AlertRegistry);
        let watcher_id = env.register_contract(None, WatcherRegistry);

        let alert_client = AlertRegistryClient::new(&env, &alert_id);
        let watcher_client = watcher_registry::WatcherRegistryClient::new(&env, &watcher_id);

        (env, alert_client, watcher_client)
    }

    // 1. Happy path — register and retrieve
    #[test]
    fn test_register_and_get_alert() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "My Alert"),
            &str(&env, "hash123"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.label, str(&env, "My Alert"));
        assert_eq!(cfg.owner, owner);
        assert!(cfg.active);
    }

    // 2. Happy path — update alert
    #[test]
    fn test_update_alert() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        assert_eq!(
            client
                .try_update_alert(&owner, &id, &vec![&env, str(&env, "rule:mint")], &false)
                .unwrap(),
            Ok(())
        );

        let cfg = client.get_alert(&id).unwrap();
        assert!(!cfg.active);
        assert_eq!(cfg.rules.get(0).unwrap(), str(&env, "rule:mint"));
    }

    // 3. Happy path — remove alert
    #[test]
    fn test_remove_alert() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        assert_eq!(client.try_remove_alert(&owner, &id).unwrap(), Ok(()));
        assert!(client.get_alert(&id).is_none());
    }

    // 4. Unauthorized update rejected
    #[test]
    fn test_update_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        assert_eq!(
            client
                .try_update_alert(&attacker, &id, &vec![&env], &false)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    #[test]
    #[should_panic(expected = "invalid rule descriptor")]
    fn test_register_alert_rejects_invalid_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:unknown")],
        );
    }

    #[test]
    #[should_panic(expected = "invalid rule descriptor")]
    fn test_update_alert_rejects_invalid_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        client.update_alert(&owner, &id, &vec![&env, str(&env, "rule:bogus")], &true);
    }

    #[test]
    fn test_admin_remove_any_alert() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin).unwrap();

        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:mint")],
        );

        assert_eq!(client.remove_alert_by_admin(&admin, &id), Ok(()));
        assert!(client.get_alert(&id).is_none());
    }

    #[test]
    #[should_panic(expected = "owner alert limit exceeded")]
    fn test_admin_set_per_owner_alert_limit() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin).unwrap();
        client.set_per_owner_alert_limit(&admin, &1u32).unwrap();

        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert1"),
            &str(&env, "hash1"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert2"),
            &str(&env, "hash2"),
            &vec![&env, str(&env, "rule:mint")],
        );
    }

    #[test]
    fn test_admin_transfer_admin() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin).unwrap();
        let new_admin = Address::generate(&env);

        assert_eq!(client.transfer_admin(&admin, &new_admin), Ok(()));
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        assert_eq!(client.remove_alert_by_admin(&new_admin, &id), Ok(()));
    }

    // 5. Unauthorized remove rejected
    #[test]
    fn test_remove_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        assert_eq!(
            client
                .try_remove_alert(&attacker, &id)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 6. Edge case — get nonexistent alert returns None
    #[test]
    fn test_get_nonexistent_alert() {
        let (_env, client) = setup();
        assert!(client.get_alert(&999u64).is_none());
    }

    // 7. Edge case — get alerts for contract with no alerts returns empty vec
    #[test]
    fn test_get_alerts_for_contract_empty() {
        let (env, client) = setup();
        let querier = Address::generate(&env);
        let target = Address::generate(&env);
        let result = client.get_alerts_for_contract(&querier, &target).unwrap();
        assert_eq!(result.len(), 0);
    }

    // 8. Index queries — get_alerts_for_contract and get_alerts_by_owner
    #[test]
    fn test_index_queries() {
        let (env, client) = setup();
        let querier = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        client.register_alert(
            &owner,
            &target,
            &str(&env, "A1"),
            &str(&env, "h1"),
            &vec![&env],
        );
        client.register_alert(
            &owner,
            &target,
            &str(&env, "A2"),
            &str(&env, "h2"),
            &vec![&env],
        );

        assert_eq!(
            client.get_alerts_for_contract(&querier, &target).unwrap().len(),
            2
        );
        assert_eq!(
            client.get_alerts_by_owner(&querier, &owner).unwrap().len(),
            2
        );
    }

    // 9. get_alert_count reflects registered alerts (monotonic — does not decrease)
    #[test]
    fn test_get_alert_count() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        assert_eq!(client.get_alert_count(), 0u64);

        client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "h"),
            &vec![&env],
        );
        assert_eq!(client.get_alert_count(), 1u64);
    }

    // 10. Paginated queries work without watcher gating
    #[test]
    fn test_paginated_queries_no_gating() {
        let (env, client) = setup();
        let querier = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        for i in 0..5u32 {
            let label = String::from_str(&env, "alert");
            let _ = i; // suppress unused warning
            client.register_alert(
                &owner,
                &target,
                &label,
                &str(&env, "h"),
                &vec![&env],
            );
        }

        let page = client
            .get_contract_alerts_paginated(&querier, &target, &0u32, &3u32)
            .unwrap();
        assert_eq!(page.len(), 3);

        let page2 = client
            .get_alerts_by_owner_paginated(&querier, &owner, &3u32, &10u32)
            .unwrap();
        assert_eq!(page2.len(), 2);
    }

    // ── Watcher-gating tests ──────────────────────────────────────────────

    // 11. No watcher registry configured — any querier can read
    #[test]
    fn test_no_watcher_registry_any_querier_can_read() {
        let (env, client) = setup();
        let stranger = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        // No registry set — stranger can still query
        assert_eq!(
            client.get_alerts_for_contract(&stranger, &target).unwrap().len(),
            1
        );
    }

    // 12. Watcher registry configured — registered watcher can read
    #[test]
    #[cfg(feature = "testutils")]
    fn test_watcher_registry_registered_watcher_can_read() {
        use watcher_registry::WatcherRegistry;
        let (env, alert_client, watcher_client) = setup_with_watcher_registry();

        let admin = Address::generate(&env);
        let watcher = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        watcher_client.initialize(&admin);
        watcher_client.register_watcher(&admin, &watcher);

        // Point alert registry at the watcher registry
        alert_client.initialize(&admin).unwrap();
        let watcher_contract_id = watcher_client.address.clone();
        alert_client
            .set_watcher_registry(&admin, &watcher_contract_id)
            .unwrap();

        alert_client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        // Registered watcher can query
        let results = alert_client
            .get_alerts_for_contract(&watcher, &target)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    // 13. Watcher registry configured — unregistered address is rejected
    #[test]
    #[cfg(feature = "testutils")]
    fn test_watcher_registry_unregistered_address_rejected() {
        use watcher_registry::WatcherRegistry;
        let (env, alert_client, watcher_client) = setup_with_watcher_registry();

        let admin = Address::generate(&env);
        let stranger = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        watcher_client.initialize(&admin);

        alert_client.initialize(&admin).unwrap();
        let watcher_contract_id = watcher_client.address.clone();
        alert_client
            .set_watcher_registry(&admin, &watcher_contract_id)
            .unwrap();

        alert_client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        // Stranger (not a watcher) is rejected
        assert_eq!(
            alert_client
                .try_get_alerts_for_contract(&stranger, &target)
                .unwrap_err()
                .unwrap(),
            ContractError::NotAWatcher
        );
    }

    // 14. Watcher registry configured — removed watcher loses access
    #[test]
    #[cfg(feature = "testutils")]
    fn test_watcher_registry_removed_watcher_loses_access() {
        use watcher_registry::WatcherRegistry;
        let (env, alert_client, watcher_client) = setup_with_watcher_registry();

        let admin = Address::generate(&env);
        let watcher = Address::generate(&env);
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        watcher_client.initialize(&admin);
        watcher_client.register_watcher(&admin, &watcher);

        alert_client.initialize(&admin).unwrap();
        let watcher_contract_id = watcher_client.address.clone();
        alert_client
            .set_watcher_registry(&admin, &watcher_contract_id)
            .unwrap();

        alert_client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        // Watcher can read before removal
        assert_eq!(
            alert_client
                .get_alerts_for_contract(&watcher, &target)
                .unwrap()
                .len(),
            1
        );

        // Remove the watcher
        watcher_client.remove_watcher(&admin, &watcher);

        // Now rejected
        assert_eq!(
            alert_client
                .try_get_alerts_for_contract(&watcher, &target)
                .unwrap_err()
                .unwrap(),
            ContractError::NotAWatcher
        );
    }

    // 15. get_watcher_registry returns None before configuration
    #[test]
    fn test_get_watcher_registry_none_before_set() {
        let (_env, client) = setup();
        assert!(client.get_watcher_registry().is_none());
    }

    // 16. set_watcher_registry persists and get_watcher_registry returns it
    #[test]
    #[cfg(feature = "testutils")]
    fn test_set_and_get_watcher_registry() {
        use watcher_registry::WatcherRegistry;
        let (env, alert_client, watcher_client) = setup_with_watcher_registry();

        let admin = Address::generate(&env);
        alert_client.initialize(&admin).unwrap();

        let watcher_contract_id = watcher_client.address.clone();
        alert_client
            .set_watcher_registry(&admin, &watcher_contract_id)
            .unwrap();

        assert_eq!(
            alert_client.get_watcher_registry().unwrap(),
            watcher_contract_id
        );
    }

    // 17. Only admin can set watcher registry
    #[test]
    fn test_set_watcher_registry_non_admin_rejected() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let attacker = Address::generate(&env);
        let fake_registry = Address::generate(&env);

        client.initialize(&admin).unwrap();

        assert_eq!(
            client
                .try_set_watcher_registry(&attacker, &fake_registry)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 18. updated_at is strictly greater than created_at after update_alert
    //
    // The Soroban test environment starts with timestamp 0 and does not
    // advance automatically. We manually bump the ledger timestamp by 1
    // second between registration and update so that the contract's
    // `env.ledger().timestamp()` call inside `update_alert` returns a
    // value that is strictly greater than the one captured at registration.
    #[test]
    fn test_updated_at_strictly_greater_than_created_at() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        // Register at timestamp T (default = 0).
        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Timestamp Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        let before = client.get_alert(&id).unwrap();
        assert_eq!(
            before.created_at, before.updated_at,
            "created_at and updated_at should be equal right after registration"
        );

        // Advance the ledger clock by 1 second so the update lands at T+1.
        env.ledger().with_mut(|li| {
            li.timestamp += 1;
        });

        client
            .update_alert(&owner, &id, &vec![&env, str(&env, "rule:mint")], &true)
            .unwrap();

        let after = client.get_alert(&id).unwrap();
        assert!(
            after.updated_at > after.created_at,
            "updated_at ({}) must be strictly greater than created_at ({})",
            after.updated_at,
            after.created_at
        );
    }

    // 19. Register an alert with exactly 50 valid rule strings.
    //
    // This verifies that the contract handles the maximum allowed rule count
    // without hitting Soroban instruction limits. We alternate between the
    // two valid rule descriptors ("rule:transfer" and "rule:mint") to fill
    // all 50 slots, then confirm every entry is stored correctly.
    #[test]
    fn test_register_alert_with_50_rules_no_instruction_limit() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        // Build a vec of 50 valid rules, alternating between the two
        // accepted descriptors so the list is realistic.
        let mut rules: Vec<String> = vec![&env];
        for i in 0..50u32 {
            let rule = if i % 2 == 0 {
                str(&env, "rule:transfer")
            } else {
                str(&env, "rule:mint")
            };
            rules.push_back(rule);
        }

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Bulk Rules Alert"),
            &str(&env, "hash"),
            &rules,
        );

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(
            cfg.rules.len(),
            50,
            "all 50 rules should be persisted"
        );

        // Spot-check a few entries to confirm data integrity.
        assert_eq!(cfg.rules.get(0).unwrap(), str(&env, "rule:transfer"));
        assert_eq!(cfg.rules.get(1).unwrap(), str(&env, "rule:mint"));
        assert_eq!(cfg.rules.get(48).unwrap(), str(&env, "rule:transfer"));
        assert_eq!(cfg.rules.get(49).unwrap(), str(&env, "rule:mint"));
    }

    // ── Feature A: update_label ───────────────────────────────────────────────

    // 18. Happy path — update_label changes only the label
    #[test]
    fn test_update_label_changes_label() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Original"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        assert_eq!(
            client.try_update_label(&owner, &id, &str(&env, "Renamed")).unwrap(),
            Ok(())
        );

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.label, str(&env, "Renamed"));
        // rules and webhook_hash must be untouched
        assert_eq!(cfg.rules.get(0).unwrap(), str(&env, "rule:transfer"));
        assert_eq!(cfg.webhook_hash, str(&env, "hash"));
        assert!(cfg.active);
    }

    // 19. update_label — unauthorized caller is rejected
    #[test]
    fn test_update_label_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        assert_eq!(
            client
                .try_update_label(&attacker, &id, &str(&env, "Hacked"))
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 20. update_label — nonexistent alert returns AlertNotFound
    #[test]
    fn test_update_label_not_found() {
        let (env, client) = setup();
        let caller = Address::generate(&env);

        assert_eq!(
            client
                .try_update_label(&caller, &999u64, &str(&env, "X"))
                .unwrap_err()
                .unwrap(),
            ContractError::AlertNotFound
        );
    }

    // 21. update_label — label exceeding 128 bytes is rejected
    #[test]
    #[should_panic(expected = "label exceeds 128 bytes")]
    fn test_update_label_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        client.update_label(&owner, &id, &str(&env, &"a".repeat(129)));
    }

    // 22. update_label — exactly 128 bytes is accepted
    #[test]
    fn test_update_label_max_length_accepted() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env],
        );

        assert_eq!(
            client
                .try_update_label(&owner, &id, &str(&env, &"a".repeat(128)))
                .unwrap(),
            Ok(())
        );
    }

    // ── Feature B: get_active_alerts_for_contract ─────────────────────────────

    // 23. Happy path — only active alerts are returned
    #[test]
    fn test_get_active_alerts_for_contract_filters_inactive() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id1 = client.register_alert(
            &owner,
            &target,
            &str(&env, "Active"),
            &str(&env, "h1"),
            &vec![&env, str(&env, "rule:transfer")],
        );
        let _id2 = client.register_alert(
            &owner,
            &target,
            &str(&env, "Inactive"),
            &str(&env, "h2"),
            &vec![&env, str(&env, "rule:mint")],
        );

        // Deactivate the second alert
        client.update_alert(&owner, &_id2, &vec![&env, str(&env, "rule:mint")], &false);

        let all = client.get_alerts_for_contract(&target);
        assert_eq!(all.len(), 2);

        let active = client.get_active_alerts_for_contract(&target);
        assert_eq!(active.len(), 1);
        assert_eq!(active.get(0).unwrap().label, str(&env, "Active"));
        let _ = id1;
    }

    // 24. get_active_alerts_for_contract — returns empty when all are inactive
    #[test]
    fn test_get_active_alerts_for_contract_all_inactive() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:transfer")],
        );

        client.update_alert(&owner, &id, &vec![&env, str(&env, "rule:transfer")], &false);

        let active = client.get_active_alerts_for_contract(&target);
        assert_eq!(active.len(), 0);
    }

    // 25. get_active_alerts_for_contract — returns empty for unknown contract
    #[test]
    fn test_get_active_alerts_for_contract_empty() {
        let (env, client) = setup();
        let target = Address::generate(&env);
        assert_eq!(client.get_active_alerts_for_contract(&target).len(), 0);
    }

    // 26. get_active_alerts_for_contract — all active alerts are returned
    #[test]
    fn test_get_active_alerts_for_contract_all_active() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        client.register_alert(
            &owner,
            &target,
            &str(&env, "A1"),
            &str(&env, "h1"),
            &vec![&env, str(&env, "rule:transfer")],
        );
        client.register_alert(
            &owner,
            &target,
            &str(&env, "A2"),
            &str(&env, "h2"),
            &vec![&env, str(&env, "rule:mint")],
        );

        let active = client.get_active_alerts_for_contract(&target);
        assert_eq!(active.len(), 2);
    }
}
