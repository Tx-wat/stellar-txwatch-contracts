use soroban_sdk::{symbol_short, vec, Address, Env, Vec};

use crate::types::{AlertConfig, DataKey};

// ── ID counter ───────────────────────────────────────────────────────────────

/// Atomically read and increment the global alert ID counter.
///
/// Returns the current value before incrementing, so the first ID is `0`.
pub fn next_id(env: &Env) -> u64 {
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

// ── Index helpers ─────────────────────────────────────────────────────────────

/// Load the list of alert IDs owned by `owner`, or an empty vec.
pub fn owner_index(env: &Env, owner: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::OwnerIndex(owner.clone()))
        .unwrap_or_else(|| vec![env])
}

/// Load the list of alert IDs watching `target`, or an empty vec.
pub fn contract_index(env: &Env, target: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::ContractIndex(target.clone()))
        .unwrap_or_else(|| vec![env])
}

/// Append `id` to the owner's index and persist it with a refreshed TTL.
///
/// Panics if `id` is already present to enforce index uniqueness.
pub fn push_owner_index(env: &Env, owner: &Address, id: u64) {
    let mut ids = owner_index(env, owner);
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
pub fn push_contract_index(env: &Env, target: &Address, id: u64) {
    let mut ids = contract_index(env, target);
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
pub fn remove_from_owner_index(env: &Env, owner: &Address, id: u64) {
    let ids = owner_index(env, owner);
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
pub fn remove_from_contract_index(env: &Env, target: &Address, id: u64) {
    let ids = contract_index(env, target);
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

// ── Config helpers ────────────────────────────────────────────────────────────

/// Resolve a list of alert IDs to their stored [`AlertConfig`] values.
///
/// IDs that no longer exist in storage (expired or removed) are silently
/// skipped. Callers that need to detect missing entries should call
/// `get_alert` per ID instead.
pub fn configs_for_ids(env: &Env, ids: &Vec<u64>) -> Vec<AlertConfig> {
    let mut out: Vec<AlertConfig> = vec![env];
    for i in 0..ids.len() {
        let id = ids.get(i).unwrap();
        if let Some(cfg) = env.storage().persistent().get(&DataKey::Alert(id)) {
            out.push_back(cfg);
        }
    }
    out
}

/// Return a paginated slice of configs from `ids` (offset + limit).
pub fn configs_paginated(
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

/// Remove an alert record from storage and clean up both indexes.
///
/// Emits an `alert/remove` event on completion.
pub fn remove_alert_record(env: &Env, config: &AlertConfig, config_id: u64, caller: &Address) {
    use soroban_sdk::symbol_short;

    env.storage()
        .persistent()
        .remove(&DataKey::Alert(config_id));

    remove_from_owner_index(env, &config.owner, config_id);
    remove_from_contract_index(env, &config.target_contract, config_id);

    env.events().publish(
        (symbol_short!("alert"), symbol_short!("remove")),
        (config_id, caller.clone()),
    );
}
