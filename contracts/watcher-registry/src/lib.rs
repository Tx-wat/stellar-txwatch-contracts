#![no_std]
#![warn(clippy::pedantic)]
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, symbol_short, vec, Address, Env, Vec};

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    NotInitialized = 3,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Storage key variants used to address instance entries.
#[contracttype]
pub enum DataKey {
    Admins,
    Watchers,
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct WatcherRegistry;

#[contractimpl]
impl WatcherRegistry {
    /// Initialize the registry with an admin address. Can only be called once.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `admin`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        if env
            .storage()
            .instance()
            .has(&symbol_short!("ADMIN"))
        {
            return Err(ContractError::AlreadyInitialized);
        }
        env.storage()
            .instance()
            .set(&symbol_short!("ADMIN"), &admin);
        Ok(())
    }

    /// Register an authorized watcher node (admin only).
    pub fn register_watcher(
        env: Env,
        admin: Address,
        watcher: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;

        let mut watchers = Self::load_watchers(&env);
        for i in 0..watchers.len() {
            if watchers.get(i).unwrap() == watcher {
                return Ok(()); // already registered, idempotent
            }
        }
        watchers.push_back(watcher.clone());
        env.storage()
            .instance()
            .set(&symbol_short!("WATCHERS"), &watchers);

        env.events().publish(
            (symbol_short!("watcher"), symbol_short!("register")),
            watcher,
        );

        Ok(())
    }

    /// Remove a watcher (admin only).
    pub fn remove_watcher(env: Env, admin: Address, watcher: Address) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;

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
        Ok(())
    }

    /// Check if an address is an authorized watcher.
    #[must_use]
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
    #[must_use]
    pub fn get_watchers(env: Env) -> Vec<Address> {
        Self::load_watchers(&env)
    }

    /// Transfer admin role to a new address (admin only).
    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;
        // Transfer replaces the admin set with a single new admin
        let admins: Vec<Address> = vec![&env, new_admin];
        env.storage().instance().set(&symbol_short!("ADMINS"), &admins);
        Ok(())
    }

    /// Remove all authorized watchers at once (admin only).
    ///
    /// This is intended for emergency deauthorization scenarios where all
    /// watcher permissions must be revoked immediately.
    pub fn clear_all_watchers(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;

        let empty: Vec<Address> = vec![&env];
        env.storage()
            .instance()
            .set(&symbol_short!("WATCHERS"), &empty);

        env.events().publish(
            (symbol_short!("watcher"), symbol_short!("clear_all")),
            (),
        );

        Ok(())
    }

    /// Get the current admin address.
    #[must_use]
    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .ok_or(ContractError::NotInitialized)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Load the current watcher list from instance storage, or return an empty vec.
    fn load_watchers(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("WATCHERS"))
            .unwrap_or_else(|| vec![env])
    }

    fn load_admins(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .map(|admin: Address| vec![env, admin])
            .unwrap_or_else(|| vec![env])
    }

    fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        let admins = Self::load_admins(env);
        for i in 0..admins.len() {
            if admins.get(i).unwrap() == *caller {
                return Ok(());
            }
        }
        Err(ContractError::Unauthorized)
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
        client.try_initialize(&admin).unwrap();
        (env, admin, client)
    }

    // 1. Happy path — register and check authorization
    #[test]
    fn test_register_and_is_authorized() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        assert!(!client.is_authorized(&watcher));
        assert_eq!(
            client.try_register_watcher(&admin, &watcher).unwrap(),
            Ok(())
        );
        assert!(client.is_authorized(&watcher));
    }

    #[test]
    #[should_panic]
    fn test_initialize_requires_admin_auth() {
        let env = Env::default();
        let contract_id = env.register_contract(None, WatcherRegistry);
        let client = WatcherRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.initialize(&admin);
    }

    // 2. Happy path — remove watcher
    #[test]
    fn test_remove_watcher() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        client.try_register_watcher(&admin, &watcher).unwrap();
        assert_eq!(client.try_remove_watcher(&admin, &watcher).unwrap(), Ok(()));
        assert!(!client.is_authorized(&watcher));
    }

    // 3. Happy path — transfer admin
    #[test]
    fn test_transfer_admin() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        assert_eq!(
            client.try_transfer_admin(&admin, &new_admin).unwrap(),
            Ok(())
        );
        // old admin can no longer register
        // new admin can register
        assert_eq!(
            client.try_register_watcher(&new_admin, &watcher).unwrap(),
            Ok(())
        );
        assert!(client.is_authorized(&watcher));
    }

    // 4. Unauthorized register rejected
    #[test]
    fn test_register_unauthorized() {
        let (env, _admin, client) = setup();
        let attacker = Address::generate(&env);
        let watcher = Address::generate(&env);

        assert_eq!(
            client
                .try_register_watcher(&attacker, &watcher)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 5. Unauthorized remove rejected
    #[test]
    fn test_remove_unauthorized() {
        let (env, admin, client) = setup();
        let attacker = Address::generate(&env);
        let watcher = Address::generate(&env);

        client.try_register_watcher(&admin, &watcher).unwrap();
        assert_eq!(
            client
                .try_remove_watcher(&attacker, &watcher)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 6. Edge case — double initialize returns AlreadyInitialized error
    #[test]
    fn test_double_initialize() {
        let (env, _admin, client) = setup();
        let other = Address::generate(&env);
        let err = client.try_initialize(&other).unwrap_err().unwrap();
        assert_eq!(err, ContractError::AlreadyInitialized);
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

        assert_eq!(
            client.try_register_watcher(&admin, &watcher).unwrap(),
            Ok(())
        );
        assert_eq!(
            client.try_register_watcher(&admin, &watcher).unwrap(),
            Ok(())
        );
        assert_eq!(client.get_watchers().len(), 1);
    }

    // 8b. Edge case — repeated calls with the same watcher stay idempotent
    #[test]
    fn test_register_idempotent_after_five_duplicates() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        for _ in 0..5 {
            assert_eq!(
                client.try_register_watcher(&admin, &watcher).unwrap(),
                Ok(())
            );
        }

        assert_eq!(client.get_watchers().len(), 1);
    }

    // 9. Multiple watchers
    #[test]
    fn test_multiple_watchers() {
        let (env, admin, client) = setup();
        let w1 = Address::generate(&env);
        let w2 = Address::generate(&env);
        let w3 = Address::generate(&env);

        client.try_register_watcher(&admin, &w1).unwrap();
        client.try_register_watcher(&admin, &w2).unwrap();
        client.try_register_watcher(&admin, &w3).unwrap();

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

    #[test]
    fn test_get_admin_uninitialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, WatcherRegistry);
        let client = WatcherRegistryClient::new(&env, &contract_id);

        assert_eq!(
            client.try_get_admin().unwrap_err().unwrap(),
            ContractError::NotInitialized
        );
    }

    // 11. get_admin panics with NotInitialized when contract is not initialized
    #[test]
    #[should_panic]
    fn test_get_admin_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, WatcherRegistry);
        let client = WatcherRegistryClient::new(&env, &contract_id);
        client.get_admin();
    }

    // 12. clear_all_watchers removes all watchers
    #[test]
    fn test_clear_all_watchers() {
        let (env, admin, client) = setup();
        let w1 = Address::generate(&env);
        let w2 = Address::generate(&env);
        let w3 = Address::generate(&env);

        client.try_register_watcher(&admin, &w1).unwrap();
        client.try_register_watcher(&admin, &w2).unwrap();
        client.try_register_watcher(&admin, &w3).unwrap();
        assert_eq!(client.get_watchers().len(), 3);

        assert_eq!(
            client.try_clear_all_watchers(&admin).unwrap(),
            Ok(())
        );
        assert_eq!(client.get_watchers().len(), 0);
        assert!(!client.is_authorized(&w1));
        assert!(!client.is_authorized(&w2));
        assert!(!client.is_authorized(&w3));
    }

    // 13. clear_all_watchers rejects non-admin
    #[test]
    fn test_clear_all_watchers_unauthorized() {
        let (env, admin, client) = setup();
        let attacker = Address::generate(&env);

        client.try_register_watcher(&admin, &Address::generate(&env)).unwrap();

        assert_eq!(
            client
                .try_clear_all_watchers(&attacker)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
        assert_eq!(client.get_watchers().len(), 1);
    }

    // 14. clear_all_watchers on empty list is a no-op (does not error)
    #[test]
    fn test_clear_all_watchers_empty() {
        let (env, admin, client) = setup();

        assert_eq!(
            client.try_clear_all_watchers(&admin).unwrap(),
            Ok(())
        );
        assert_eq!(client.get_watchers().len(), 0);
    }

    // 15. old admin cannot act after transfer
    #[test]
    fn test_old_admin_rejected_after_transfer() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        assert_eq!(
            client.try_transfer_admin(&admin, &new_admin).unwrap(),
            Ok(())
        );
        assert_eq!(
            client
                .try_register_watcher(&admin, &watcher)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }
}
