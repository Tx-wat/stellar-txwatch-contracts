#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, vec, Address, Env, Vec};

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Storage key variants used to address instance entries.
#[contracttype]
pub enum DataKey {
    /// Stores the current admin [`Address`].
    Admin,
    /// Stores the list of authorized watcher [`Address`] values.
    Watchers,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct WatcherRegistry;

#[contractimpl]
impl WatcherRegistry {
    /// Initialize the registry with an admin address. Can only be called once.
    pub fn initialize(env: Env, admin: Address) {
        if env
            .storage()
            .instance()
            .has(&symbol_short!("ADMIN"))
        {
            panic!("already initialized");
        }
        env.storage()
            .instance()
            .set(&symbol_short!("ADMIN"), &admin);
    }

    /// Register an authorized watcher node (admin only).
    pub fn register_watcher(env: Env, admin: Address, watcher: Address) {
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let mut watchers = Self::load_watchers(&env);
        for i in 0..watchers.len() {
            if watchers.get(i).unwrap() == watcher {
                return; // already registered, idempotent
            }
        }
        watchers.push_back(watcher);
        env.storage()
            .instance()
            .set(&symbol_short!("WATCHERS"), &watchers);
    }

    /// Remove a watcher (admin only).
    pub fn remove_watcher(env: Env, admin: Address, watcher: Address) {
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let watchers = Self::load_watchers(&env);
        let mut updated: Vec<Address> = vec![&env];
        for i in 0..watchers.len() {
            let w = watchers.get(i).unwrap();
            if w != watcher {
                updated.push_back(w);
            }
        }
        env.storage()
            .instance()
            .set(&symbol_short!("WATCHERS"), &updated);
    }

    /// Check if an address is an authorized watcher.
    pub fn is_authorized(env: Env, watcher: Address) -> bool {
        let watchers = Self::load_watchers(&env);
        for i in 0..watchers.len() {
            if watchers.get(i).unwrap() == watcher {
                return true;
            }
        }
        false
    }

    /// Get all authorized watcher addresses.
    pub fn get_watchers(env: Env) -> Vec<Address> {
        Self::load_watchers(&env)
    }

    /// Transfer admin role to a new address (admin only).
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        admin.require_auth();
        Self::assert_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&symbol_short!("ADMIN"), &new_admin);
    }

    /// Get the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .expect("not initialized")
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Load the current watcher list from instance storage, or return an empty vec.
    fn load_watchers(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("WATCHERS"))
            .unwrap_or_else(|| vec![env])
    }

    /// Assert that `caller` is the current admin.
    ///
    /// # Panics
    /// Panics with `"not initialized"` if the registry has not been initialized.
    /// Panics with `"unauthorized"` if `caller` is not the admin.
    fn assert_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .expect("not initialized");
        if admin != *caller {
            panic!("unauthorized");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, Address, WatcherRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, WatcherRegistry);
        let client = WatcherRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    // 1. Happy path — register and check authorization
    #[test]
    fn test_register_and_is_authorized() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        assert!(!client.is_authorized(&watcher));
        client.register_watcher(&admin, &watcher);
        assert!(client.is_authorized(&watcher));
    }

    // 2. Happy path — remove watcher
    #[test]
    fn test_remove_watcher() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        client.register_watcher(&admin, &watcher);
        client.remove_watcher(&admin, &watcher);
        assert!(!client.is_authorized(&watcher));
    }

    // 3. Happy path — transfer admin
    #[test]
    fn test_transfer_admin() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin);
        // old admin can no longer register
        // new admin can register
        client.register_watcher(&new_admin, &watcher);
        assert!(client.is_authorized(&watcher));
    }

    // 4. Unauthorized register rejected
    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_register_unauthorized() {
        let (env, _admin, client) = setup();
        let attacker = Address::generate(&env);
        let watcher = Address::generate(&env);
        client.register_watcher(&attacker, &watcher);
    }

    // 5. Unauthorized remove rejected
    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_remove_unauthorized() {
        let (env, admin, client) = setup();
        let attacker = Address::generate(&env);
        let watcher = Address::generate(&env);

        client.register_watcher(&admin, &watcher);
        client.remove_watcher(&attacker, &watcher);
    }

    // 6. Edge case — double initialize panics
    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize() {
        let (env, _admin, client) = setup();
        let other = Address::generate(&env);
        client.initialize(&other);
    }

    // 7. Edge case — get_watchers returns empty before any registration
    #[test]
    fn test_get_watchers_empty() {
        let (_env, _admin, client) = setup();
        assert_eq!(client.get_watchers().len(), 0);
    }

    // 8. Edge case — register same watcher twice is idempotent
    #[test]
    fn test_register_idempotent() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        client.register_watcher(&admin, &watcher);
        client.register_watcher(&admin, &watcher);
        assert_eq!(client.get_watchers().len(), 1);
    }

    // 9. Multiple watchers
    #[test]
    fn test_multiple_watchers() {
        let (env, admin, client) = setup();
        let w1 = Address::generate(&env);
        let w2 = Address::generate(&env);
        let w3 = Address::generate(&env);

        client.register_watcher(&admin, &w1);
        client.register_watcher(&admin, &w2);
        client.register_watcher(&admin, &w3);

        assert_eq!(client.get_watchers().len(), 3);
        assert!(client.is_authorized(&w1));
        assert!(client.is_authorized(&w2));
        assert!(client.is_authorized(&w3));
    }

    // 10. get_admin returns correct admin
    #[test]
    fn test_get_admin() {
        let (_env, admin, client) = setup();
        assert_eq!(client.get_admin(), admin);
    }

    // 11. old admin cannot act after transfer
    #[test]
    #[should_panic(expected = "unauthorized")]
    fn test_old_admin_rejected_after_transfer() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);
        let watcher = Address::generate(&env);
        client.transfer_admin(&admin, &new_admin);
        client.register_watcher(&admin, &watcher);
    }
}
