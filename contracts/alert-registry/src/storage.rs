use soroban_sdk::{symbol_short, vec, Address, Env, Vec};

use crate::types::{AlertConfig, ContractError, DataKey};
use crate::{DEFAULT_TTL};

// ── ID counter ────────────────────────────────────────────────────────────────

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

// ── Alert CRUD ────────────────────────────────────────────────────────────────

pub fn get_alert(env: &Env, id: u64) -> Option<AlertConfig> {
    env.storage().persistent().get(&DataKey::Alert(id))
}

pub fn set_alert(env: &Env, id: u64, config: &AlertConfig) {
    env.storage().persistent().set(&DataKey::Alert(id), config);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Alert(id), DEFAULT_TTL, DEFAULT_TTL);
}

pub fn remove_alert(env: &Env, id: u64) {
    env.storage().persistent().remove(&DataKey::Alert(id));
}

pub fn has_alert(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Alert(id))
}

/// Extend the TTL of an alert entry without modifying its data.
pub fn extend_alert_ttl(env: &Env, id: u64) {
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Alert(id), 100, 100);
}

// ── Owner index ───────────────────────────────────────────────────────────────

/// Load the list of alert IDs owned by `owner`, or an empty vec.
pub fn owner_index(env: &Env, owner: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::OwnerIndex(owner.clone()))
        .unwrap_or_else(|| vec![env])
}

/// Append `id` to the owner's index and persist it with a refreshed TTL.
///
/// Panics if `id` is already present to enforce index uniqueness.
pub fn push_owner_index(env: &Env, owner: &Address, id: u64) -> Result<(), ContractError> {
    let mut ids = owner_index(env, owner);
    for i in 0..ids.len() {
        if ids.get(i).unwrap() == id {
            return Err(ContractError::DuplicateAlertId);
        }
    }
    ids.push_back(id);
    env.storage()
        .persistent()
        .set(&DataKey::OwnerIndex(owner.clone()), &ids);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::OwnerIndex(owner.clone()), DEFAULT_TTL, DEFAULT_TTL);
    Ok(())
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
        .extend_ttl(&DataKey::OwnerIndex(owner.clone()), DEFAULT_TTL, DEFAULT_TTL);
}

/// Extend the TTL of the owner index without modifying its data.
pub fn extend_owner_index_ttl(env: &Env, owner: &Address) {
    if env
        .storage()
        .persistent()
        .has(&DataKey::OwnerIndex(owner.clone()))
    {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::OwnerIndex(owner.clone()), 100, 100);
    }
}

// ── Contract index ────────────────────────────────────────────────────────────

/// Load the list of alert IDs watching `target`, or an empty vec.
pub fn contract_index(env: &Env, target: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::ContractIndex(target.clone()))
        .unwrap_or_else(|| vec![env])
}

/// Append `id` to the contract's index and persist it with a refreshed TTL.
///
/// Panics if `id` is already present to enforce index uniqueness.
pub fn push_contract_index(env: &Env, target: &Address, id: u64) -> Result<(), ContractError> {
    let mut ids = contract_index(env, target);
    for i in 0..ids.len() {
        if ids.get(i).unwrap() == id {
            return Err(ContractError::DuplicateAlertId);
        }
    }
    ids.push_back(id);
    env.storage()
        .persistent()
        .set(&DataKey::ContractIndex(target.clone()), &ids);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::ContractIndex(target.clone()), DEFAULT_TTL, DEFAULT_TTL);
    Ok(())
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
        .extend_ttl(&DataKey::ContractIndex(target.clone()), DEFAULT_TTL, DEFAULT_TTL);
}

/// Extend the TTL of the contract index without modifying its data.
pub fn extend_contract_index_ttl(env: &Env, target: &Address) {
    if env
        .storage()
        .persistent()
        .has(&DataKey::ContractIndex(target.clone()))
    {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::ContractIndex(target.clone()), 100, 100);
    }
}

// ── Batch reads ───────────────────────────────────────────────────────────────

/// Resolve a list of alert IDs to their stored [`AlertConfig`] values.
///
/// IDs that no longer exist in storage (expired or removed) are silently
/// skipped — the returned vec may be shorter than `ids`.
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

pub fn configs_paginated(env: &Env, ids: &Vec<u64>, offset: u32, limit: u32) -> Vec<AlertConfig> {
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

// ── Admin / limit ─────────────────────────────────────────────────────────────

pub fn get_admin(env: &Env) -> Option<soroban_sdk::Address> {
    env.storage().instance().get(&symbol_short!("ADMIN"))
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&symbol_short!("ADMIN"), admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&symbol_short!("ADMIN"))
}

pub fn get_limit(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("LIMIT"))
        .unwrap_or(0u32)
}

pub fn set_limit(env: &Env, limit: u32) {
    env.storage().instance().set(&symbol_short!("LIMIT"), &limit);
}
