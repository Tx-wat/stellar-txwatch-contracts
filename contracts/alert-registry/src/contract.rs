//! Secondary (non-active) AlertRegistry implementation.
//!
//! This module contains a clean, modular implementation of the AlertRegistry
//! contract logic backed by the `storage` and `types` modules.  It is **not**
//! the compiled contract entry-point — that role belongs to `lib.rs`.
//!
//! The struct and impl are kept as plain Rust (no `#[contract]` /
//! `#[contractimpl]` attributes) so they compile without generating a
//! duplicate Soroban client and without conflicting with the `lib.rs`
//! implementation.  The `tests.rs` file registers the `lib.rs` version.

#![allow(dead_code)]

use soroban_sdk::{symbol_short, Address, Env, String, Vec};

use crate::storage;
use crate::types::{AlertConfig, AlertInput, ContractError, MAX_BATCH_SIZE};

pub struct AlertRegistry;

impl AlertRegistry {
    /// Register a new alert config and return its assigned ID.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `owner`.
    ///
    /// # Events
    /// Emits `(Symbol("alert"), Symbol("register"))` with data
    /// `(id: u64, owner: Address, target_contract: Address)`.
    pub fn register_alert(
        env: Env,
        owner: Address,
        target_contract: Address,
        label: String,
        webhook_hash: String,
        rules: Vec<String>,
    ) -> Result<u64, ContractError> {
        owner.require_auth();

        if label.len() > 128 {
            return Err(ContractError::LabelTooLong);
        }

        validate_rules(&env, &rules)?;
        assert_per_owner_limit(&env, &owner)?;

        let id = storage::next_id(&env);
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
            pending_webhook_hash: None,
        };

        storage::set_alert(&env, id, &config);
        storage::push_owner_index(&env, &owner, id)?;
        storage::push_contract_index(&env, &target_contract, id)?;

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("register")),
            (id, owner, target_contract),
        );

        id
    }

    /// Update the rules and active flag of an existing alert.
    pub fn update_alert(
        env: Env,
        owner: Address,
        config_id: u64,
        rules: Vec<String>,
        active: bool,
    ) -> Result<(), ContractError> {
        owner.require_auth();

        let mut config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &owner)?;
        validate_rules(&env, &rules);

        config.rules = rules;
        config.active = active;
        config.updated_at = env.ledger().timestamp();

        storage::set_alert(&env, config_id, &config);
        Ok(())
    }

    /// Update the webhook hash for an existing alert.
    pub fn update_webhook(
        env: Env,
        caller: Address,
        config_id: u64,
        webhook_hash: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;

        config.webhook_hash = webhook_hash;
        config.updated_at = env.ledger().timestamp();

        storage::set_alert(&env, config_id, &config);
        Ok(())
    }

    /// Propose a new webhook hash for an existing alert (step 1 of 2).
    ///
    /// The new hash is stored in `pending_webhook_hash` and does **not** replace
    /// the live `webhook_hash` yet. The owner must call [`confirm_webhook`] to
    /// complete the rotation. This two-step flow prevents a window where the old
    /// webhook is deactivated before the new one is confirmed by the off-chain
    /// watcher.
    ///
    /// Calling `propose_webhook` again before confirming overwrites the previous
    /// pending hash, allowing the owner to correct a mistake.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Errors
    /// Returns [`ContractError::AlertNotFound`] if `config_id` does not exist.
    /// Returns [`ContractError::Unauthorized`] if `caller` is not the alert owner.
    ///
    /// # Events
    /// Emits `(Symbol("alert"), Symbol("wh_prop"))` with data `(id: u64, caller: Address)`.
    pub fn propose_webhook(
        env: Env,
        caller: Address,
        config_id: u64,
        new_webhook_hash: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;

        config.pending_webhook_hash = Some(new_webhook_hash);
        config.updated_at = env.ledger().timestamp();

        storage::set_alert(&env, config_id, &config);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("wh_prop")),
            (config_id, caller),
        );

        Ok(())
    }

    /// Confirm a pending webhook hash rotation (step 2 of 2).
    ///
    /// Promotes `pending_webhook_hash` to `webhook_hash` and clears the pending
    /// field. The caller must have previously called [`propose_webhook`] for this
    /// alert. Returns [`ContractError::NoPendingWebhook`] if no rotation is in
    /// progress.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Errors
    /// Returns [`ContractError::AlertNotFound`] if `config_id` does not exist.
    /// Returns [`ContractError::Unauthorized`] if `caller` is not the alert owner.
    /// Returns [`ContractError::NoPendingWebhook`] if no pending hash exists.
    ///
    /// # Events
    /// Emits `(Symbol("alert"), Symbol("wh_conf"))` with data `(id: u64, caller: Address)`.
    pub fn confirm_webhook(
        env: Env,
        caller: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;

        let pending = config
            .pending_webhook_hash
            .take()
            .ok_or(ContractError::NoPendingWebhook)?;

        config.webhook_hash = pending;
        config.updated_at = env.ledger().timestamp();

        storage::set_alert(&env, config_id, &config);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("wh_conf")),
            (config_id, caller),
        );

        Ok(())
    }

    /// Extend the TTL of an alert and its index entries without modifying any data.
    ///
    /// This is the recommended way to keep an alert alive without triggering an
    /// `updated_at` change (which would cause it to appear in incremental-sync
    /// results). Call this periodically from an off-chain keeper to prevent
    /// storage archival.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Errors
    /// Returns [`ContractError::AlertNotFound`] if `config_id` does not exist.
    /// Returns [`ContractError::Unauthorized`] if `caller` is not the alert owner.
    pub fn renew_alert_ttl(
        env: Env,
        caller: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;

        // Extend the alert entry itself.
        storage::extend_alert_ttl(&env, config_id);
        // Extend both index entries so they stay alive as long as the alert.
        storage::extend_owner_index_ttl(&env, &config.owner);
        storage::extend_contract_index_ttl(&env, &config.target_contract);

        Ok(())
    }

    /// Remove an alert config from storage.
    pub fn remove_alert(
        env: Env,
        caller: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;
        remove_alert_record(&env, &config, config_id, &caller);
        Ok(())
    }

    /// Retrieve a single alert config by its ID.
    pub fn get_alert(env: Env, config_id: u64) -> Option<AlertConfig> {
        storage::get_alert(&env, config_id)
    }

    /// Initialize the optional admin role for the registry.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if storage::has_admin(&env) {
            return Err(ContractError::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Transfer the admin role to a new address (admin only).
    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        assert_admin(&env, &admin)?;
        storage::set_admin(&env, &new_admin);
        Ok(())
    }

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        storage::get_admin(&env).ok_or(ContractError::NotInitialized)
    }

    /// Set a per-owner active alert limit (admin only).
    pub fn set_per_owner_alert_limit(
        env: Env,
        admin: Address,
        limit: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        assert_admin(&env, &admin)?;
        storage::set_limit(&env, limit);
        Ok(())
    }

    /// Get the configured per-owner active alert limit, or `0` if none is set.
    pub fn get_per_owner_alert_limit(env: Env) -> u32 {
        storage::get_limit(&env)
    }

    /// Remove any alert config from storage (admin only).
    pub fn remove_alert_by_admin(
        env: Env,
        admin: Address,
        config_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        assert_admin(&env, &admin)?;

        let config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        remove_alert_record(&env, &config, config_id, &admin);
        Ok(())
    }

    /// Retrieve all alert configs that watch a given contract address.
    pub fn get_alerts_for_contract(env: Env, target_contract: Address) -> Vec<AlertConfig> {
        let ids = storage::contract_index(&env, &target_contract);
        storage::configs_for_ids(&env, &ids)
    }

    /// Retrieve all alert configs owned by a given address.
    pub fn get_alerts_by_owner(env: Env, owner: Address) -> Vec<AlertConfig> {
        let ids = storage::owner_index(&env, &owner);
        storage::configs_for_ids(&env, &ids)
    }

    /// Get a page of alert configs for a target contract (offset + limit).
    pub fn get_contract_alerts_paginated(
        env: Env,
        target_contract: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = storage::contract_index(&env, &target_contract);
        storage::configs_paginated(&env, &ids, offset, limit)
    }

    /// Get a page of alert configs owned by an address (offset + limit).
    pub fn get_alerts_by_owner_paginated(
        env: Env,
        owner: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = storage::owner_index(&env, &owner);
        storage::configs_paginated(&env, &ids, offset, limit)
    }

    /// Get the total number of alerts ever registered (monotonic counter).
    #[must_use]
    pub fn get_alert_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::NextId)
            .unwrap_or(0u64)
    }

    /// Get the number of currently active (non-removed) alerts owned by `owner`.
    pub fn get_active_alert_count(env: Env, owner: Address) -> u32 {
        let ids = storage::owner_index(&env, &owner);
        let mut count: u32 = 0;
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if storage::has_alert(&env, id) {
                count += 1;
            }
        }
        count
    }

    /// Return the count of live (non-removed) alert configs watching `target_contract`.
    ///
    /// Unlike [`get_alert_count`], this is scoped to a single watched contract and
    /// reflects removals — it only counts entries that still exist in storage.
    pub fn get_alert_count_for_contract(env: Env, target_contract: Address) -> u32 {
        let ids = storage::contract_index(&env, &target_contract);
        let mut count: u32 = 0;
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if storage::has_alert(&env, id) {
                count += 1;
            }
        }
        count
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn assert_owner(config: &AlertConfig, caller: &Address) -> Result<(), ContractError> {
    if config.owner == *caller {
        Ok(())
    } else {
        Err(ContractError::Unauthorized)
    }
}

fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    match storage::get_admin(env) {
        Some(admin) if admin == *caller => Ok(()),
        Some(_) => Err(ContractError::Unauthorized),
        None => Err(ContractError::NotInitialized),
    }
}

fn assert_per_owner_limit(env: &Env, owner: &Address) -> Result<(), ContractError> {
    let limit = storage::get_limit(env);
    if limit > 0 {
        let ids = storage::owner_index(env, owner);
        let mut count: u32 = 0;
        for i in 0..ids.len() {
            if storage::has_alert(env, ids.get(i).unwrap()) {
                count += 1;
            }
        }
        if count >= limit {
            return Err(ContractError::OwnerAlertLimitExceeded);
        }
    }
    Ok(())
}

fn validate_rules(env: &Env, rules: &Vec<String>) -> Result<(), ContractError> {
    if rules.len() > 50 {
        return Err(ContractError::TooManyRules);
    }
    let transfer = String::from_str(env, "rule:transfer");
    let mint = String::from_str(env, "rule:mint");
    for i in 0..rules.len() {
        let rule = rules.get(i).unwrap();
        if rule != transfer && rule != mint {
            return Err(ContractError::InvalidRuleDescriptor);
        }
    }
    Ok(())
}

fn remove_alert_record(env: &Env, config: &AlertConfig, config_id: u64, caller: &Address) {
    storage::remove_alert(env, config_id);
    storage::remove_from_owner_index(env, &config.owner, config_id);
    storage::remove_from_contract_index(env, &config.target_contract, config_id);

    env.events().publish(
        (symbol_short!("alert"), symbol_short!("remove")),
        (config_id, caller.clone()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env, String};

    fn setup() -> (Env, AlertRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(AlertRegistry, ());
        let client = AlertRegistryClient::new(&env, &contract_id);
        (env, client)
    }

    fn str(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    fn register(client: &AlertRegistryClient, env: &Env, owner: &Address, target: &Address) -> u64 {
        client
            .register_alert(
                owner,
                target,
                &str(env, "alert"),
                &str(env, "hash"),
                &vec![env],
            )
            .unwrap()
    }

    #[test]
    fn test_count_zero_for_unknown_contract() {
        let (env, client) = setup();
        let target = Address::generate(&env);
        assert_eq!(client.get_alert_count_for_contract(&target), 0u32);
    }

    #[test]
    fn test_count_increments_on_register() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        assert_eq!(client.get_alert_count_for_contract(&target), 0u32);
        register(&client, &env, &owner, &target);
        assert_eq!(client.get_alert_count_for_contract(&target), 1u32);
        register(&client, &env, &owner, &target);
        assert_eq!(client.get_alert_count_for_contract(&target), 2u32);
    }

    #[test]
    fn test_count_decrements_on_remove() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        assert_eq!(client.get_alert_count_for_contract(&target), 1u32);
        client.remove_alert(&owner, &id).unwrap();
        assert_eq!(client.get_alert_count_for_contract(&target), 0u32);
    }

    #[test]
    fn test_count_isolated_per_contract() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target_a = Address::generate(&env);
        let target_b = Address::generate(&env);

        register(&client, &env, &owner, &target_a);
        register(&client, &env, &owner, &target_a);
        register(&client, &env, &owner, &target_b);

        assert_eq!(client.get_alert_count_for_contract(&target_a), 2u32);
        assert_eq!(client.get_alert_count_for_contract(&target_b), 1u32);
    }
}
