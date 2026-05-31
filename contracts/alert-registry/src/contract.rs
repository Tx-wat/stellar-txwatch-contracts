use soroban_sdk::{contract, contractimpl, panic_with_error, symbol_short, Address, Env, String, Vec};

use crate::storage::{
    configs_for_ids, configs_paginated, contract_index, next_id, owner_index,
    push_contract_index, push_owner_index, remove_alert_record,
};
use crate::types::{AlertConfig, ContractError, DataKey};

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

        Self::validate_rules(&env, &rules);

        Self::assert_per_owner_limit(&env, &owner);

        let id = next_id(&env);
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
        push_owner_index(&env, &owner, id);
        push_contract_index(&env, &target_contract, id);

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

        remove_alert_record(&env, &config, config_id, &caller);
        Ok(())
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
        env.storage()
            .instance()
            .set(&symbol_short!("ADMIN"), &new_admin);
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
        env.storage()
            .instance()
            .set(&symbol_short!("LIMIT"), &limit);
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

        remove_alert_record(&env, &config, config_id, &admin);
        Ok(())
    }

    /// Retrieve all alert configs that watch a given contract address.
    pub fn get_alerts_for_contract(env: Env, target_contract: Address) -> Vec<AlertConfig> {
        let ids = contract_index(&env, &target_contract);
        configs_for_ids(&env, &ids)
    }

    /// Retrieve all alert configs owned by a given address.
    pub fn get_alerts_by_owner(env: Env, owner: Address) -> Vec<AlertConfig> {
        let ids = owner_index(&env, &owner);
        configs_for_ids(&env, &ids)
    }

    /// Get a page of alert configs for a target contract (offset + limit).
    pub fn get_contract_alerts_paginated(
        env: Env,
        target_contract: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = contract_index(&env, &target_contract);
        configs_paginated(&env, &ids, offset, limit)
    }

    /// Get a page of alert configs owned by an address (offset + limit).
    pub fn get_alerts_by_owner_paginated(
        env: Env,
        owner: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<AlertConfig> {
        let ids = owner_index(&env, &owner);
        configs_paginated(&env, &ids, offset, limit)
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
        let ids = owner_index(&env, &owner);
        let mut count: u32 = 0;
        for i in 0..ids.len() {
            let id = ids.get(i).unwrap();
            if env.storage().persistent().has(&DataKey::Alert(id)) {
                count += 1;
            }
        }
        count
    }

    // ── Private helpers ───────────────────────────────────────────────────────

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
        let limit: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("LIMIT"))
            .unwrap_or(0u32);
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
}
