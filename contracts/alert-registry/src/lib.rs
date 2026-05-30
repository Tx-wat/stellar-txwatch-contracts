#![no_std]
#![warn(clippy::pedantic)]
use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, symbol_short, vec, Address, Env, String, Vec,
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
/// # Storage and TTL
/// All persistent entries are extended by 100 ledgers (~8 minutes) on every
/// write. See `docs/ttl.md` for implications and how to tune this value.
#[contract]
pub struct AlertRegistry;

#[contractimpl]
impl AlertRegistry {
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
    /// Requires a valid Stellar auth signature from `owner`, who must also be
    /// the original owner of the alert.
    ///
    /// # Panics
    /// Panics with `"alert not found"` if `config_id` does not exist.
    /// Panics with `"unauthorized"` if `owner` is not the alert owner.
    pub fn update_alert(
        env: Env,
        owner: Address,
        config_id: u64,
        rules: Vec<String>,
        active: bool,
    ) -> Result<(), ContractError> {
        owner.require_auth();

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::assert_owner(&config, &owner)?;

        Self::validate_rules(&env, &rules);

        config.rules = rules;
        config.active = active;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage().persistent().extend_ttl(&DataKey::Alert(config_id), 100, 100);
        env.storage().persistent().extend_ttl(&DataKey::OwnerIndex(config.owner.clone()), 100, 100);
        env.storage().persistent().extend_ttl(&DataKey::ContractIndex(config.target_contract.clone()), 100, 100);
        Ok(())
    }

    /// Update the webhook hash for an existing alert.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `owner`, who must also be
    /// the original owner of the alert.
    ///
    /// # Panics
    /// Panics with `"alert not found"` if `config_id` does not exist.
    /// Panics with `"unauthorized"` if `caller` is not the alert owner.
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

        Self::assert_owner(&config, &owner)?;

        config.webhook_hash = webhook_hash;
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
    /// Requires a valid Stellar auth signature from `owner`, who must also be
    /// the original owner of the alert.
    ///
    /// # Panics
    /// Panics with `"alert not found"` if `config_id` does not exist.
    /// Panics with `"unauthorized"` if `caller` is not the alert owner.
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

        Self::assert_owner(&config, &owner)?;

        env.storage()
            .persistent()
            .remove(&DataKey::Alert(config_id));

        Self::remove_from_owner_index(&env, &owner, config_id);
        Self::remove_from_contract_index(&env, &config.target_contract, config_id);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("remove")),
            (config_id, caller),
        );

        Ok(())
    }

    fn assert_owner(config: &AlertConfig, owner: &Address) -> Result<(), ContractError> {
        if config.owner == *owner {
            Ok(())
        } else {
            Err(ContractError::Unauthorized)
        }
    }

    /// Retrieve a single alert config by its ID.
    ///
    /// Returns `None` if the alert does not exist or has expired.
    pub fn get_alert(env: Env, config_id: u64) -> Option<AlertConfig> {
        env.storage().persistent().get(&DataKey::Alert(config_id))
    }

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

    /// Remove any alert config from storage (admin only).
    pub fn remove_alert_by_admin(
        env: Env,
        admin: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;

        let config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .ok_or(ContractError::AlertNotFound)?;

        Self::remove_alert_record(&env, &config, config_id, &admin);
        Ok(())
    }

    /// Retrieve all alert configs that watch a given contract address.
    ///
    /// Returns an empty vec if no alerts are registered for `target_contract`.
    pub fn get_alerts_for_contract(env: Env, target_contract: Address) -> Vec<AlertConfig> {
        let ids = Self::contract_index(&env, &target_contract);
        Self::configs_for_ids(&env, &ids)
    }

    /// Retrieve all alert configs owned by a given address.
    ///
    /// Returns an empty vec if `owner` has no registered alerts.
    pub fn get_alerts_by_owner(env: Env, owner: Address) -> Vec<AlertConfig> {
        let ids = Self::owner_index(&env, &owner);
        Self::configs_for_ids(&env, &ids)
    }

    /// Get a page of alert configs for a target contract (offset + limit).
    pub fn get_contract_alerts_paginated(
        env: Env,
        target_contract: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = Self::contract_index(&env, &target_contract);
        Self::configs_paginated(&env, &ids, offset, limit)
    }

    /// Get a page of alert configs owned by an address (offset + limit).
    pub fn get_alerts_by_owner_paginated(
        env: Env,
        owner: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = Self::owner_index(&env, &owner);
        Self::configs_paginated(&env, &ids, offset, limit)
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
        let limit = Self::get_per_owner_alert_limit(env);
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

    // ── Internal helpers ─────────────────────────────────────────────────────

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
    ///
    /// Panics if `id` is already present to enforce index uniqueness.
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
    ///
    /// Panics if `id` is already present to enforce index uniqueness.
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
    ///
    /// Rebuilds the index by copying all IDs except `id`. The TTL is refreshed
    /// on the updated entry.
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
    ///
    /// Rebuilds the index by copying all IDs except `id`. The TTL is refreshed
    /// on the updated entry.
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
    /// IDs that no longer exist in storage (expired or removed) are **silently
    /// skipped** — the returned vec may be shorter than `ids`. Callers that
    /// need to detect missing entries should call [`get_alert`] per ID instead.
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
            &vec![&env, str(&env, "rule:a")],
        );

        assert_eq!(
            client
                .try_update_alert(&owner, &id, &vec![&env, str(&env, "rule:b")], &false)
                .unwrap(),
            Ok(())
        );

        let cfg = client.get_alert(&id).unwrap();
        assert!(!cfg.active);
        assert_eq!(cfg.rules.get(0).unwrap(), str(&env, "rule:b"));
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

    #[test]
    fn test_old_admin_rejected_after_transfer() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin).unwrap();
        let new_admin = Address::generate(&env);

        // first transfer succeeds
        assert_eq!(
            client.try_transfer_admin(&admin, &new_admin).unwrap(),
            Ok(())
        );

        // old admin cannot call transfer_admin again
        assert_eq!(
            client
                .try_transfer_admin(&admin, &new_admin)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
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
        let target = Address::generate(&env);
        let result = client.get_alerts_for_contract(&target);
        assert_eq!(result.len(), 0);
    }

    // 8. Index queries — get_alerts_for_contract and get_alerts_by_owner
    #[test]
    fn test_index_queries() {
        let (env, client) = setup();
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

        assert_eq!(client.get_alerts_for_contract(&target).len(), 2);
        assert_eq!(client.get_alerts_by_owner(&owner).len(), 2);
    }

    // 9. get_alert_count reflects registered alerts (monotonic — does not decrease)
    #[test]
    fn test_get_alert_count() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        assert_eq!(client.get_alert_count(), 0);
        let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
        assert_eq!(client.get_alert_count(), 1);
        client.register_alert(
            &owner,
            &target,
            &str(&env, "B"),
            &str(&env, "h"),
            &vec![&env],
        );
        assert_eq!(client.get_alert_count(), 2);
        // monotonic: removing does not decrease the counter
        client.remove_alert(&owner, &id);
        assert_eq!(client.get_alert_count(), 2);
    }

    // Issue #2 — get_active_alert_count decreases after remove
    #[test]
    fn test_get_active_alert_count() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        assert_eq!(client.get_active_alert_count(&owner), 0);
        let id1 = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
        let _id2 = client.register_alert(&owner, &target, &str(&env, "B"), &str(&env, "h"), &vec![&env]);
        assert_eq!(client.get_active_alert_count(&owner), 2);
        client.remove_alert(&owner, &id1);
        assert_eq!(client.get_active_alert_count(&owner), 1);
    }

    // 10. update_webhook changes the hash
    #[test]
    fn test_update_webhook() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "old-hash"),
            &vec![&env],
        );
        assert_eq!(
            client
                .try_update_webhook(&owner, &id, &str(&env, "new-hash"))
                .unwrap(),
            Ok(())
        );
        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.webhook_hash, str(&env, "new-hash"));
    }

    // 11. update_webhook unauthorized
    #[test]
    fn test_update_webhook_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "hash"),
            &vec![&env],
        );
        assert_eq!(
            client
                .try_update_webhook(&attacker, &id, &str(&env, "evil-hash"))
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    #[test]
    fn test_update_alert_missing_returns_not_found() {
        let (env, client) = setup();
        let attacker = Address::generate(&env);

        assert_eq!(
            client
                .try_update_alert(&attacker, &999u64, &vec![&env], &false)
                .unwrap_err()
                .unwrap(),
            ContractError::AlertNotFound
        );
    }

    // 12. Issue #65 — active defaults to true on registration
    #[test]
    fn test_active_defaults_to_true() {
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

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.active, true);
    }

    // 13. Issue #115 — register_alert rejects more than 50 rules
    #[test]
    #[should_panic(expected = "too many rules: maximum is 50")]
    fn test_register_alert_too_many_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let mut rules: Vec<String> = vec![&env];
        for _ in 0..51u32 {
            rules.push_back(String::from_str(&env, &soroban_sdk::String::from_str(&env, "rule").to_string()));
        }
        client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
    }

    // 14. Issue #115 — update_alert rejects more than 50 rules
    #[test]
    #[should_panic(expected = "too many rules: maximum is 50")]
    fn test_update_alert_too_many_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "A"),
            &str(&env, "h"),
            &vec![&env],
        );

        let mut rules: Vec<String> = vec![&env];
        for _ in 0..51u32 {
            rules.push_back(String::from_str(&env, &soroban_sdk::String::from_str(&env, "rule").to_string()));
        }
        client.update_alert(&owner, &id, &rules, &true);
    }

    // 15. Issue #115 — exactly 50 rules is accepted
    #[test]
    fn test_register_alert_exactly_50_rules() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let mut rules: Vec<String> = vec![&env];
        for _ in 0..50u32 {
            rules.push_back(str(&env, "rule"));
        }
        let id = client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &rules);
        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.rules.len(), 50);
    }

    // 16. Label exceeding 128 bytes is rejected
    #[test]
    #[should_panic(expected = "label exceeds 128 bytes")]
    fn test_label_too_long() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let long_label = str(&env, &"a".repeat(129));
        client.register_alert(
            &owner,
            &target,
            &long_label,
            &str(&env, "hash"),
            &vec![&env],
        );
    }

    // 17. Label at exactly 128 bytes is accepted
    #[test]
    fn test_label_max_length_accepted() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);
        let max_label = str(&env, &"a".repeat(128));
        client.register_alert(&owner, &target, &max_label, &str(&env, "hash"), &vec![&env]);
    }

    // 18. get_admin panics with NotInitialized when contract is not initialized
    #[test]
    #[should_panic]
    fn test_get_admin_not_initialized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AlertRegistry);
        let client = AlertRegistryClient::new(&env, &contract_id);
        client.get_admin();
    }

    // 19. Alert can be deactivated and reactivated via update_alert
    #[test]
    fn test_alert_deactivate_reactivate() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner,
            &target,
            &str(&env, "Alert"),
            &str(&env, "hash"),
            &vec![&env, str(&env, "rule:mint")],
        );

        // deactivate
        assert_eq!(
            client
                .try_update_alert(&owner, &id, &vec![&env, str(&env, "rule:mint")], &false)
                .unwrap(),
            Ok(())
        );
        let cfg = client.get_alert(&id).unwrap();
        assert!(!cfg.active);

        // reactivate
        assert_eq!(
            client
                .try_update_alert(&owner, &id, &vec![&env, str(&env, "rule:mint")], &true)
                .unwrap(),
            Ok(())
        );
        let cfg = client.get_alert(&id).unwrap();
        assert!(cfg.active);
    }
}
