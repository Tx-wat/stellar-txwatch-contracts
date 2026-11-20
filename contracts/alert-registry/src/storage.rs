use soroban_sdk::{symbol_short, vec, Address, Env, Vec};

use crate::types::{AlertConfig, ContractError, DataKey};
use crate::{DEFAULT_TTL, RATE_LIMIT_COUNT, RATE_LIMIT_WINDOW};

// ── ID counter ────────────────────────────────────────────────────────────────

/// Atomically read and increment the global alert ID counter.
///
/// Returns the current value before incrementing, so the first ID is `0`.
/// Stored in persistent storage so it survives instance TTL expiry.
///
/// The counter is monotonic and non-reusable: IDs are never recycled, even
/// after an alert is removed. If the counter is already at `u64::MAX`, the
/// increment would overflow (wrapping to `0` and overwriting alert ID `0` in
/// release builds), so this returns [`ContractError::CounterOverflow`]
/// instead of wrapping.
pub fn next_id(env: &Env) -> Result<u64, ContractError> {
    let id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::NextId)
        .unwrap_or(0u64);
    let next = id.checked_add(1).ok_or(ContractError::CounterOverflow)?;
    env.storage().persistent().set(&DataKey::NextId, &next);
    // Use DEFAULT_TTL so the counter survives as long as the alert data it
    // belongs to.  The previous hard-coded 100-ledger bump was far too short —
    // a contract that only receives TTL renewals (no new registrations) would
    // let NextId expire and then restart from 0 on the next registration,
    // producing ID collisions.  See issue #212.
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::NextId, DEFAULT_TTL, DEFAULT_TTL);
    Ok(id)
}

/// Extend the TTL of the global alert ID counter to [`DEFAULT_TTL`].
///
/// Callers that keep alert data alive (e.g. the `bump_alert` entrypoint, see
/// [`crate::lib::bump_alert`]) **must** also call this function so that the
/// `NextId` counter cannot expire independently of the alert entries it tracks.
///
/// If `NextId` expires while alert data is still live the next call to
/// `register_alert` would restart the counter from `0`, silently reusing IDs
/// that are already in use and causing ID collisions (see issue #212).
///
/// This is a no-op when `NextId` is not yet in storage (counter never
/// initialised), so it is safe to call unconditionally.
pub fn extend_next_id_ttl(env: &Env) {
    if env.storage().persistent().has(&DataKey::NextId) {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::NextId, DEFAULT_TTL, DEFAULT_TTL);
    }
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

/// Extend the TTL of an alert entry and its indexes without modifying data.
///
/// Returns `true` if the alert still exists (and was renewed), `false` if it
/// was already removed or expired.
pub fn bulk_renew_one(env: &Env, id: u64) -> bool {
    let Some(cfg) = env.storage().persistent().get::<_, AlertConfig>(&DataKey::Alert(id)) else {
        return false;
    };
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Alert(id), 100, 100);
    extend_owner_index_ttl(env, &cfg.owner);
    extend_contract_index_ttl(env, &cfg.target);
    true
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

pub fn get_limit(env: &Env) -> Option<u32> {
    env.storage()
        .instance()
        .get(&symbol_short!("LIMIT"))
}

pub fn set_limit(env: &Env, limit: u32) {
    env.storage().instance().set(&symbol_short!("LIMIT"), &limit);
}

// ── Pause / circuit breaker ───────────────────────────────────────────────────

/// Returns `true` if the contract is currently paused.
pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&symbol_short!("PAUSED"))
        .unwrap_or(false)
}

/// Persist the paused flag in instance storage.
pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&symbol_short!("PAUSED"), &paused);
}

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Enforce the per-owner registration rate limit.
///
/// Tracks the current window's start ledger and the number of registrations
/// made within it. Once `RATE_LIMIT_COUNT` registrations have occurred inside
/// a `RATE_LIMIT_WINDOW`-ledger window, further registrations are rejected
/// with [`ContractError::RateLimitExceeded`] until the window rolls over.
pub fn check_rate_limit(env: &Env, owner: &Address) -> Result<(), ContractError> {
    if is_paused(env) {
        return Err(ContractError::ContractPaused);
    }

    let now = env.ledger().sequence();
    let key = DataKey::RateLimit(owner.clone());
    let (window_start, count): (u32, u32) = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or((now, 0u32));

    if now.saturating_sub(window_start) >= RATE_LIMIT_WINDOW {
        env.storage().persistent().set(&key, &(now, 1u32));
        env.storage()
            .persistent()
            .extend_ttl(&key, DEFAULT_TTL, DEFAULT_TTL);
        return Ok(());
    }

    if count >= RATE_LIMIT_COUNT {
        return Err(ContractError::RateLimitExceeded);
    }

    env.storage().persistent().set(&key, &(window_start, count + 1));
    env.storage()
        .persistent()
        .extend_ttl(&key, DEFAULT_TTL, DEFAULT_TTL);
    Ok(())
}

