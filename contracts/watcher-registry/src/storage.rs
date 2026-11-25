//! Storage helpers for the watcher registry.
//!
//! All reads and writes to instance and persistent storage go through these
//! functions so that key layout and TTL policy live in one place.

use soroban_sdk::{Address, Env, Map, Vec};

/// Default TTL (in ledgers) used when extending instance storage.
/// ~30 days at 5s/ledger.
///
/// Deliberately longer than the `alert-registry` default (`120_960`, ~7 days):
/// the watcher set changes infrequently, while alert data is written often.
/// See `docs/ttl.md`.
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
        .persistent()
        .get(&DataKey::Watcher(watcher.clone()))
}

/// Returns true when the watcher exists and has not expired at the current
/// ledger timestamp.
pub fn is_watcher_authorized(env: &Env, watcher: &Address) -> bool {
    match get_watcher(env, watcher) {
        Some(record) => match record.expires_at {
            Some(exp) => env.ledger().timestamp() <= exp,
            None => true,
        },
        None => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban::Env;

    fn setup() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    #[test]
    fn update_target_contract_migrates_contract_index() {
        let env = setup();
        let contract_a = Address::generate(&env);
        let contract_b = Address::generate(&env);
        let alert_id = 1u64;

        // Register an alert watching contract_a.
        let mut ids_a = Vec::new(&env);
        ids_a.push_back(alert_id);
        env.storage()
            .instance()
            .set(&DataKey::AlertIdsForContract(contract_a.clone()), &ids_a);

        // Migrate the alert to contract_b: remove from old index, add to new.
        let mut ids_a = env
            .storage()
            .instance()
            .get::<_, Vec<u64>>(&DataKey::AlertIdsForContract(contract_a.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        let mut remaining = Vec::new(&env);
        for id in ids_a.iter() {
            if id != alert_id {
                remaining.push_back(id);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::AlertIdsForContract(contract_a.clone()), &remaining);

        let mut ids_b = env
            .storage()
            .instance()
            .get::<_, Vec<u64>>(&DataKey::AlertIdsForContract(contract_b.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        ids_b.push_back(alert_id);
        env.storage()
            .instance()
            .set(&DataKey::AlertIdsForContract(contract_b.clone()), &ids_b);

        // Old contract index must be empty.
        let old_ids = env
            .storage()
            .instance()
            .get::<_, Vec<u64>>(&DataKey::AlertIdsForContract(contract_a.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        assert_eq!(old_ids.len(), 0);

        // New contract index must contain the alert ID.
        let new_ids = env
            .storage()
            .instance()
            .get::<_, Vec<u64>>(&DataKey::AlertIdsForContract(contract_b.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        assert_eq!(new_ids.len(), 1);
        assert_eq!(new_ids.get(0).unwrap(), alert_id);
    }
}

pub fn save_info(env: &Env, watcher: &Address, info: &WatcherInfo) {
    env.storage()
        .persistent()
        .set(&DataKey::WatcherInfo(watcher.clone()), info);
}

pub fn load_info(env: &Env, watcher: &Address) -> Option<WatcherInfo> {
    env.storage()
        .persistent()
        .get(&DataKey::WatcherInfo(watcher.clone()))
}

pub fn remove_info(env: &Env, watcher: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::WatcherInfo(watcher.clone()));
}
