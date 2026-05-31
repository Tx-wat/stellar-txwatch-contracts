use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Vec};

use crate::storage;
use crate::types::{AlertConfig, ContractError};

#[contract]
pub struct AlertRegistry;

#[contractimpl]
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
    ) -> u64 {
        owner.require_auth();

        if label.len() > 128 {
            panic!("label exceeds 128 bytes");
        }

        validate_rules(&env, &rules);
        assert_per_owner_limit(&env, &owner);

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
        };

        storage::set_alert(&env, id, &config);
        storage::push_owner_index(&env, &owner, id);
        storage::push_contract_index(&env, &target_contract, id);

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
    /// # Events
    /// Planned: emits `(Symbol("alert"), Symbol("update"))` with data
    /// `(id: u64, owner: Address, active: bool)`.
    /// See `docs/events.md` for the full spec.
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

        // TODO(events): emit (Symbol("alert"), Symbol("update")),
        //               data = (config_id, owner, active)
        //               See docs/events.md — alert.update

        Ok(())
    }

    /// Update the webhook hash for an existing alert.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Events
    /// Planned: emits `(Symbol("alert"), Symbol("webhook"))` with data
    /// `(id: u64, caller: Address)`.
    /// See `docs/events.md` for the full spec.
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

        // TODO(events): emit (Symbol("alert"), Symbol("webhook")),
        //               data = (config_id, caller)
        //               See docs/events.md — alert.webhook

        Ok(())
    }

    /// Remove an alert config from storage.
    ///
    /// Also removes the alert ID from the owner and contract indexes.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must also be
    /// the original owner of the alert.
    ///
    /// # Events
    /// Emits `(Symbol("alert"), Symbol("remove"))` with data
    /// `(id: u64, caller: Address)`.
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
    ///
    /// Returns `None` if the alert does not exist or has expired.
    pub fn get_alert(env: Env, config_id: u64) -> Option<AlertConfig> {
        storage::get_alert(&env, config_id)
    }

    /// Initialize the optional admin role for the registry. Can only be called once.
    ///
    /// # Events
    /// Planned: emits `(Symbol("admin"), Symbol("init"))` with data
    /// `(admin: Address)`.
    /// See `docs/events.md` for the full spec.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if storage::has_admin(&env) {
            return Err(ContractError::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);

        // TODO(events): emit (Symbol("admin"), Symbol("init")),
        //               data = (admin)
        //               See docs/events.md — admin.init

        Ok(())
    }

    /// Transfer the admin role to a new address (admin only).
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `admin`.
    ///
    /// # Events
    /// Planned: emits `(Symbol("admin"), Symbol("transfer"))` with data
    /// `(old_admin: Address, new_admin: Address)`.
    /// See `docs/events.md` for the full spec.
    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        assert_admin(&env, &admin)?;
        storage::set_admin(&env, &new_admin);

        // TODO(events): emit (Symbol("admin"), Symbol("transfer")),
        //               data = (admin, new_admin)
        //               See docs/events.md — admin.transfer

        Ok(())
    }

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        storage::get_admin(&env).ok_or(ContractError::NotInitialized)
    }

    /// Set a per-owner active alert limit (admin only). A value of `0` means no limit.
    ///
    /// # Events
    /// Planned: emits `(Symbol("admin"), Symbol("limit"))` with data
    /// `(admin: Address, limit: u32)`.
    /// See `docs/events.md` for the full spec.
    pub fn set_per_owner_alert_limit(
        env: Env,
        admin: Address,
        limit: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        assert_admin(&env, &admin)?;
        storage::set_limit(&env, limit);

        // TODO(events): emit (Symbol("admin"), Symbol("limit")),
        //               data = (admin, limit)
        //               See docs/events.md — admin.limit

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
            .instance()
            .get(&symbol_short!("NEXT_ID"))
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

    /// Transfer ownership of alert `config_id` from `caller` to `new_owner`.
    ///
    /// Updates both the `Alert` config entry and the `OwnerIndex` entries for
    /// the previous and new owners atomically. The `ContractIndex` is unchanged
    /// because the watched contract address does not change.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller` (the current owner).
    ///
    /// # Errors
    /// - `AlertNotFound` — if `config_id` does not exist in storage.
    /// - `Unauthorized` — if `caller` is not the current owner.
    ///
    /// # Panics
    /// Panics if `new_owner == caller` (no-op transfer is rejected).
    pub fn transfer_alert_ownership(
        env: Env,
        caller: Address,
        config_id: u64,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        assert!(
            new_owner != caller,
            "new_owner must differ from the current owner"
        );

        let mut config = storage::get_alert(&env, config_id)
            .ok_or(ContractError::AlertNotFound)?;

        assert_owner(&config, &caller)?;

        let previous_owner = config.owner.clone();
        storage::remove_from_owner_index(&env, &previous_owner, config_id);
        config.owner = new_owner.clone();
        storage::push_owner_index(&env, &new_owner, config_id);
        storage::set_alert(&env, config_id, &config);

        env.events().publish(
            (symbol_short!("alert"), symbol_short!("transfer")),
            (config_id, previous_owner, new_owner),
        );

        Ok(())
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

fn assert_per_owner_limit(env: &Env, owner: &Address) {
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
            panic!("owner alert limit exceeded");
        }
    }
}

fn validate_rules(env: &Env, rules: &Vec<String>) {
    if rules.len() > 50 {
        panic!("too many rules: maximum is 50");
    }
    let transfer = String::from_str(env, "rule:transfer");
    let mint = String::from_str(env, "rule:mint");
    for i in 0..rules.len() {
        let rule = rules.get(i).unwrap();
        if rule != transfer && rule != mint {
            panic!("invalid rule descriptor");
        }
    }
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
        client.register_alert(
            owner,
            target,
            &str(env, "alert"),
            &str(env, "hash"),
            &vec![env],
        )
    }

    #[test]
    fn test_transfer_updates_owner_field() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        client.transfer_alert_ownership(&owner, &id, &new_owner).unwrap();

        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.owner, new_owner);
    }

    #[test]
    fn test_transfer_updates_owner_index() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        client.transfer_alert_ownership(&owner, &id, &new_owner).unwrap();

        // old owner no longer has the alert
        assert_eq!(client.get_alerts_by_owner(&owner).len(), 0);
        // new owner has the alert
        assert_eq!(client.get_alerts_by_owner(&new_owner).len(), 1);
    }

    #[test]
    fn test_transfer_does_not_change_contract_index() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let target = Address::generate(&env);

        let _id = register(&client, &env, &owner, &target);
        client.transfer_alert_ownership(&owner, &_id, &new_owner).unwrap();

        // ContractIndex is unaffected
        assert_eq!(client.get_alerts_for_contract(&target).len(), 1);
    }

    #[test]
    fn test_transfer_not_found_returns_error() {
        let (env, client) = setup();
        let caller = Address::generate(&env);
        let new_owner = Address::generate(&env);

        let result = client.try_transfer_alert_ownership(&caller, &999u64, &new_owner);
        assert_eq!(result.unwrap_err().unwrap(), ContractError::AlertNotFound);
    }

    #[test]
    fn test_transfer_unauthorized_returns_error() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        let result = client.try_transfer_alert_ownership(&attacker, &id, &new_owner);
        assert_eq!(result.unwrap_err().unwrap(), ContractError::Unauthorized);
    }

    #[test]
    #[should_panic(expected = "must differ")]
    fn test_transfer_to_same_owner_panics() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        client.transfer_alert_ownership(&owner, &id, &owner).unwrap();
    }

    #[test]
    fn test_new_owner_can_update_after_transfer() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let new_owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = register(&client, &env, &owner, &target);
        client.transfer_alert_ownership(&owner, &id, &new_owner).unwrap();

        // new_owner can now update the alert
        let result = client.try_update_alert(&new_owner, &id, &vec![&env], &false);
        assert_eq!(result.unwrap(), Ok(()));
    }
}
