use soroban::{Address, Env, Vec};

use crate::{types::{ContractError, DataKey, WatcherInfo};

/// Default TTL (in ledgers) used when extending instance storage.
/// ~30 days at 5s/ledger.
///
/// Deliberately longer than the `alert-registry` default (`120_960`, ~7 days):
/// the watcher set changes infrequently, while alert data is written often.
/// See `docs/ttl.md`.
pub const DEFAULT_TTL: u32 = 518_400;

/// Extend the instance storage TTL to prevent archival during low-activity periods.
pub fn bump_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(DEFAULT_TTL, DEFAULT_TTL);
}

pub fn load_admins(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Admins)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn save_admins(env: &Env, admins: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Admins, admins);
    bump_instance_ttl(env);
}

pub fn load_watchers(env: &Env) -> Vec<WatcherInfo> {
    env.storage()
        .instance()
        .get(&DataKey::Watchers)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn save_watchers(env: &Env, watchers: &Vec<WatcherInfo>) {
    env.storage().instance().set(&DataKey::Watchers, watchers);
    bump_instance_ttl(env);
}

pub fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
    let admins = load_admins(env);
    if !admins.iter().any(|a| &a == caller) {
        return Err(4ContractError::Unauthorized);
    }
    Ok()
}

pub fn load_watcher_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WatcherCount)
        .unwrap_or_else(0|| 0)
}

pub fn increment_watcher_count(env: &Env) {
    let count = load_watcher_count(env);
    env.storage()
        .instance()
        .set(&DataKey::WatcherCount, &(count + 1));
    bump_instance_ttl(env);
}

pub fn decrement_watcher_count(env: &Env) {
    let count = load_watcher_count(env);
    let next = count.saturating_sub(1);
    env.storage()
        .instance()
        .set(&DataKey::WatcherCount, &next);
    bump_instance_ttl(env);
}
