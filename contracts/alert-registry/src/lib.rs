#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Env, String, Vec,
};

// ── Storage keys ────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Alert(u64),
    OwnerIndex(Address),
    ContractIndex(Address),
    NextId,
}

// ── Data types ───────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct AlertConfig {
    pub label: String,
    pub webhook_hash: String,
    pub rules: Vec<String>,
    pub owner: Address,
    pub target_contract: Address,
    pub created_at: u64,
    pub updated_at: u64,
    pub active: bool,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct AlertRegistry;

#[contractimpl]
impl AlertRegistry {
    /// Register a new alert config. Returns the new config ID.
    pub fn register_alert(
        env: Env,
        owner: Address,
        target_contract: Address,
        label: String,
        webhook_hash: String,
        rules: Vec<String>,
    ) -> u64 {
        owner.require_auth();

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
        env.storage().persistent().extend_ttl(&DataKey::Alert(id), 100, 100);
        Self::push_owner_index(&env, &owner, id);
        Self::push_contract_index(&env, &target_contract, id);

        id
    }

    /// Update rules and active flag of an existing alert (owner only).
    pub fn update_alert(
        env: Env,
        caller: Address,
        config_id: u64,
        rules: Vec<String>,
        active: bool,
    ) {
        caller.require_auth();

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .expect("alert not found");

        if config.owner != caller {
            panic!("unauthorized");
        }

        config.rules = rules;
        config.active = active;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage().persistent().extend_ttl(&DataKey::Alert(config_id), 100, 100);
    }

    /// Update the webhook hash for an existing alert (owner only).
    pub fn update_webhook(env: Env, caller: Address, config_id: u64, webhook_hash: String) {
        caller.require_auth();

        let mut config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .expect("alert not found");

        if config.owner != caller {
            panic!("unauthorized");
        }

        config.webhook_hash = webhook_hash;
        config.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::Alert(config_id), &config);
        env.storage().persistent().extend_ttl(&DataKey::Alert(config_id), 100, 100);
    }

    /// Remove an alert config (owner only).
    pub fn remove_alert(env: Env, caller: Address, config_id: u64) {
        caller.require_auth();

        let config: AlertConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Alert(config_id))
            .expect("alert not found");

        if config.owner != caller {
            panic!("unauthorized");
        }

        env.storage()
            .persistent()
            .remove(&DataKey::Alert(config_id));

        Self::remove_from_owner_index(&env, &caller, config_id);
        Self::remove_from_contract_index(&env, &config.target_contract, config_id);
    }

    /// Get a specific alert config by ID.
    pub fn get_alert(env: Env, config_id: u64) -> Option<AlertConfig> {
        env.storage().persistent().get(&DataKey::Alert(config_id))
    }

    /// Get all alert configs for a target contract address.
    pub fn get_alerts_for_contract(env: Env, target_contract: Address) -> Vec<AlertConfig> {
        let ids = Self::contract_index(&env, &target_contract);
        Self::configs_for_ids(&env, &ids)
    }

    /// Get all alert configs owned by an address.
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

    /// Get the total number of alerts ever registered (monotonic counter).
    pub fn get_alert_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&symbol_short!("NEXT_ID"))
            .unwrap_or(0u64)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

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

    fn owner_index(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerIndex(owner.clone()))
            .unwrap_or_else(|| vec![env])
    }

    fn contract_index(env: &Env, target: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ContractIndex(target.clone()))
            .unwrap_or_else(|| vec![env])
    }

    fn push_owner_index(env: &Env, owner: &Address, id: u64) {
        let mut ids = Self::owner_index(env, owner);
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerIndex(owner.clone()), &ids);
        env.storage().persistent().extend_ttl(&DataKey::OwnerIndex(owner.clone()), 100, 100);
    }

    fn push_contract_index(env: &Env, target: &Address, id: u64) {
        let mut ids = Self::contract_index(env, target);
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::ContractIndex(target.clone()), &ids);
        env.storage().persistent().extend_ttl(&DataKey::ContractIndex(target.clone()), 100, 100);
    }

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
        env.storage().persistent().extend_ttl(&DataKey::OwnerIndex(owner.clone()), 100, 100);
    }

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
        env.storage().persistent().extend_ttl(&DataKey::ContractIndex(target.clone()), 100, 100);
    }

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

    fn configs_paginated(env: &Env, ids: &Vec<u64>, offset: u32, limit: u32) -> Vec<AlertConfig> {
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

        client.update_alert(
            &owner,
            &id,
            &vec![&env, str(&env, "rule:b")],
            &false,
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

        client.remove_alert(&owner, &id);
        assert!(client.get_alert(&id).is_none());
    }

    // 4. Unauthorized update rejected
    #[test]
    #[should_panic(expected = "unauthorized")]
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

        client.update_alert(&attacker, &id, &vec![&env], &false);
    }

    // 5. Unauthorized remove rejected
    #[test]
    #[should_panic(expected = "unauthorized")]
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

        client.remove_alert(&attacker, &id);
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

    // 9. get_alert_count reflects registered alerts
    #[test]
    fn test_get_alert_count() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        assert_eq!(client.get_alert_count(), 0);
        client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
        assert_eq!(client.get_alert_count(), 1);
        client.register_alert(&owner, &target, &str(&env, "B"), &str(&env, "h"), &vec![&env]);
        assert_eq!(client.get_alert_count(), 2);
    }

    // 10. update_webhook changes the hash
    #[test]
    fn test_update_webhook() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner, &target, &str(&env, "A"), &str(&env, "old-hash"), &vec![&env],
        );
        client.update_webhook(&owner, &id, &str(&env, "new-hash"));
        let cfg = client.get_alert(&id).unwrap();
        assert_eq!(cfg.webhook_hash, str(&env, "new-hash"));
    }

    // 11. update_webhook unauthorized
    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_update_webhook_unauthorized() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attacker = Address::generate(&env);
        let target = Address::generate(&env);

        let id = client.register_alert(
            &owner, &target, &str(&env, "A"), &str(&env, "hash"), &vec![&env],
        );
        client.update_webhook(&attacker, &id, &str(&env, "evil-hash"));
    }

    // 12. Paginated — get_alerts_for_contract_paginated basic slicing
    #[test]
    fn test_get_alerts_for_contract_paginated() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        for _ in 0..5 {
            client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
        }

        assert_eq!(client.get_contract_alerts_paginated(&target, &0, &3).len(), 3);
        assert_eq!(client.get_contract_alerts_paginated(&target, &3, &3).len(), 2);
        assert_eq!(client.get_contract_alerts_paginated(&target, &5, &3).len(), 0);
    }

    // 13. Paginated — get_alerts_by_owner_paginated basic slicing
    #[test]
    fn test_get_alerts_by_owner_paginated() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let target = Address::generate(&env);

        for _ in 0..4 {
            client.register_alert(&owner, &target, &str(&env, "A"), &str(&env, "h"), &vec![&env]);
        }

        assert_eq!(client.get_alerts_by_owner_paginated(&owner, &0, &2).len(), 2);
        assert_eq!(client.get_alerts_by_owner_paginated(&owner, &2, &2).len(), 2);
        assert_eq!(client.get_alerts_by_owner_paginated(&owner, &4, &2).len(), 0);
    }
}
