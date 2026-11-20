//! Storage helpers for the watcher registry.
//!
//! All reads and writes to instance and persistent storage go through these
//! functions so that key layout and TTL policy live in one place.

use soroban_sdk::{Address, Env, Map, Vec};

use crate::types::{ContractError, DataKey, WatcherInfo};

/// Number of ledgers to extend storage TTL by on each access.
/// At ~5s per ledger this is roughly 30 days.
pub const DEFAULT_TTL: u32 = 518_400;

/// Extend the instance storage TTL so a dormant registry is not archived.
pub fn extend_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(DEFAULT_TTL, DEFAULT_TTL);
}

/// Load the admin set, or an empty vec when uninitialized.
pub fn load_admins(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Admins)
        .unwrap_or_else(|| Vec::new(env))
}

/// Load the authorized watcher list, or an empty vec.
pub fn load_watchers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Watchers)
        .unwrap_or_else(|| Vec::new(env))
}

/// Return `Ok(())` if `caller` is in the admin set.
///
/// # Errors
/// [`ContractError::NotInitialized`] if no admin set exists, and
/// [`ContractError::Unauthorized`] if `caller` is not an admin.
pub fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    if !env.storage().instance().has(&DataKey::Admins) {
        return Err(ContractError::NotInitialized);
    }
    let admins = load_admins(env);
    for i in 0..admins.len() {
        if admins.get(i).unwrap() == *caller {
            return Ok(());
        }
    }
    Err(ContractError::Unauthorized)
}

/// Load the current watcher count.
pub fn load_watcher_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WatcherCount)
        .unwrap_or(0u32)
}

/// Increment the watcher count by one.
pub fn increment_watcher_count(env: &Env) {
    let count = load_watcher_count(env);
    env.storage()
        .instance()
        .set(&DataKey::WatcherCount, &(count + 1));
}

/// Decrement the watcher count by one, saturating at zero.
pub fn decrement_watcher_count(env: &Env) {
    let count = load_watcher_count(env);
    env.storage()
        .instance()
        .set(&DataKey::WatcherCount, &count.saturating_sub(1));
}

/// Load the watcher expiry map (`watcher -> expires_at`).
pub fn load_expiries(env: &Env) -> Map<Address, u64> {
    env.storage()
        .instance()
        .get(&DataKey::Expiries)
        .unwrap_or_else(|| Map::new(env))
}

/// Set (`Some`) or clear (`None`) a watcher's expiry timestamp.
pub fn set_expiry(env: &Env, watcher: &Address, expires_at: Option<u64>) {
    let mut m = load_expiries(env);
    match expires_at {
        Some(exp) => m.set(watcher.clone(), exp),
        None => {
            m.remove(watcher.clone());
        }
    }
    env.storage().instance().set(&DataKey::Expiries, &m);
}

/// Persist a [`WatcherInfo`] record and extend its TTL.
pub fn save_info(env: &Env, info: &WatcherInfo) {
    let key = DataKey::WatcherInfo(info.address.clone());
    env.storage().persistent().set(&key, info);
    env.storage()
        .persistent()
        .extend_ttl(&key, DEFAULT_TTL, DEFAULT_TTL);
}
