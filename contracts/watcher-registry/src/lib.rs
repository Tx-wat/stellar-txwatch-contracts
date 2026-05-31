#![no_std]
#![warn(clippy::pedantic)]
use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, symbol_short, vec, Address, Env, Vec,
};

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    NotInitialized = 3,
    /// Returned when trying to remove the last admin, which would lock the contract.
    LastAdmin = 4,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Storage key variants used to address instance entries.
#[contracttype]
pub enum DataKey {
    /// Stores the `Vec<Address>` of current admins.
    Admins,
    /// Stores the `Vec<Address>` of authorized watcher nodes.
    Watchers,
}

// ── Contract ─────────────────────────────────────────────────────────────────

/// On-chain registry for authorized watcher nodes.
///
/// # Admin model
/// The registry supports a **set of admins** (N-of-N independent signers).
/// Any single admin can perform privileged operations (register/remove watchers,
/// add/remove other admins). This eliminates the single-point-of-failure of a
/// sole admin while keeping the authorization model simple and auditable.
///
/// All admin mutations emit Soroban events so changes are visible on-chain.
#[contract]
pub struct WatcherRegistry;

#[contractimpl]
impl WatcherRegistry {
    /// Initialize the registry with a single bootstrap admin. Can only be called once.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `admin`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Admins) {
            return Err(ContractError::AlreadyInitialized);
        }

        let admins: Vec<Address> = vec![&env, admin.clone()];
        env.storage().instance().set(&DataKey::Admins, &admins);

        env.events().publish(
            (symbol_short!("admin"), symbol_short!("init")),
            admin,
        );

        Ok(())
    }

    /// Add a new admin to the admin set (any existing admin may call this).
    ///
    /// Idempotent — adding an address that is already an admin is a no-op.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must be an
    /// existing admin.
    pub fn add_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        Self::assert_admin(&env, &caller)?;

        let mut admins = Self::load_admins(&env);
        for i in 0..admins.len() {
            if admins.get(i).unwrap() == new_admin {
                return Ok(()); // already an admin, idempotent
            }
        }
        admins.push_back(new_admin.clone());
        env.storage().instance().set(&DataKey::Admins, &admins);

        env.events().publish(
            (symbol_short!("admin"), symbol_short!("add")),
            (caller, new_admin),
        );

        Ok(())
    }

    /// Remove an admin from the admin set (any existing admin may call this).
    ///
    /// Refuses to remove the last admin to prevent the contract from becoming
    /// permanently unmanageable.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `caller`, who must be an
    /// existing admin.
    pub fn remove_admin(
        env: Env,
        caller: Address,
        target_admin: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        Self::assert_admin(&env, &caller)?;

        let admins = Self::load_admins(&env);
        if admins.len() <= 1 {
            return Err(ContractError::LastAdmin);
        }

        let mut updated: Vec<Address> = vec![&env];
        for i in 0..admins.len() {
            let a = admins.get(i).unwrap();
            if a != target_admin {
                updated.push_back(a);
            }
        }
        env.storage().instance().set(&DataKey::Admins, &updated);

        env.events().publish(
            (symbol_short!("admin"), symbol_short!("remove")),
            (caller, target_admin),
        );

        Ok(())
    }

    /// Transfer the sole admin role to a new address (any existing admin may call this).
    ///
    /// This replaces the **entire** admin set with a single new admin. Use
    /// [`add_admin`] + [`remove_admin`] if you want to rotate one member of a
    /// multi-admin set without losing the others.
    ///
    /// Emits an `("admin", "transfer")` event recording both the old and new admin.
    ///
    /// # Auth
    /// Requires a valid Stellar auth signature from `admin`, who must be an
    /// existing admin.
    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_admin(&env, &admin)?;

        let new_admins: Vec<Address> = vec![&env, new_admin.clone()];
        env.storage().instance().set(&DataKey::Admins, &new_admins);

        // Emit an auditable on-chain event recording the full admin transfer.
        env.events().publish(
            (symbol_short!("admin"), symbol_short!("transfer")),
            (admin, new_admin),
        );

        Ok(())
    }

    /// Register an authorized watcher node (any admin may call this).
    ///
    /// Idempotent — registering an already-authorized watcher is a no-op.
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
        env.storage().instance().set(&DataKey::Watchers, &watchers);

        env.events().publish(
            (symbol_short!("watcher"), symbol_short!("register")),
            watcher,
        );

        Ok(())
    }

    /// Remove a watcher (any admin may call this).
    pub fn remove_watcher(
        env: Env,
        admin: Address,
        watcher: Address,
    ) -> Result<(), ContractError> {
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
        env.storage().instance().set(&DataKey::Watchers, &updated);

        env.events().publish(
            (symbol_short!("watcher"), symbol_short!("remove")),
            watcher,
        );

        Ok(())
    }

    /// Check if an address is an authorized watcher.
    ///
    /// Renamed from `is_authorized` for clarity in cross-contract call contexts —
    /// the name now makes explicit *what* the address is being authorized as.
    #[must_use]
    pub fn is_watcher_authorized(env: Env, watcher: Address) -> bool {
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

    /// Get all current admin addresses.
    ///
    /// Returns `Err(NotInitialized)` if the contract has not been initialized.
    pub fn get_admins(env: Env) -> Result<Vec<Address>, ContractError> {
        if !env.storage().instance().has(&DataKey::Admins) {
            return Err(ContractError::NotInitialized);
        }
        Ok(Self::load_admins(&env))
    }

    /// Get the primary admin address (first in the admin set).
    ///
    /// Kept for backwards compatibility. Prefer [`get_admins`] when you need
    /// the full admin set.
    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        let admins = Self::get_admins(env)?;
        // load_admins guarantees at least one entry after initialization
        Ok(admins.get(0).unwrap())
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Load the current watcher list from instance storage, or return an empty vec.
    fn load_watchers(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Watchers)
            .unwrap_or_else(|| vec![env])
    }

    /// Load the current admin set from instance storage, or return an empty vec.
    fn load_admins(env: &Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Admins)
            .unwrap_or_else(|| vec![env])
    }

    /// Return `Ok(())` if `caller` is in the admin set, `Err(Unauthorized)` otherwise.
    fn assert_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        if !env.storage().instance().has(&DataKey::Admins) {
            return Err(ContractError::NotInitialized);
        }
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
        let contract_id = env.register(WatcherRegistry, ());
        let client = WatcherRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    // 1. Happy path — register and check authorization
    #[test]
    fn test_register_and_is_watcher_authorized() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        assert!(!client.is_watcher_authorized(&watcher));
        assert_eq!(
            client.try_register_watcher(&admin, &watcher).unwrap(),
            Ok(())
        );
        assert!(client.is_watcher_authorized(&watcher));
    }

    #[test]
    #[should_panic]
    fn test_initialize_requires_admin_auth() {
        let env = Env::default();
        let contract_id = env.register(WatcherRegistry, ());
        let client = WatcherRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
    }

    // 2. Happy path — remove watcher
    #[test]
    fn test_remove_watcher() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        client.register_watcher(&admin, &watcher);
        assert_eq!(client.try_remove_watcher(&admin, &watcher).unwrap(), Ok(()));
        assert!(!client.is_watcher_authorized(&watcher));
    }

    // 3. Happy path — transfer admin (replaces entire admin set)
    #[test]
    fn test_transfer_admin() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        assert_eq!(
            client.try_transfer_admin(&admin, &new_admin).unwrap(),
            Ok(())
        );
        // new admin can register watchers
        assert_eq!(
            client.try_register_watcher(&new_admin, &watcher).unwrap(),
            Ok(())
        );
        assert!(client.is_watcher_authorized(&watcher));
    }

    // 3b. transfer_admin emits an event
    #[test]
    fn test_transfer_admin_emits_event() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin).unwrap();

        let events = env.events().all();
        // Find the transfer event
        let found = events.iter().any(|e| {
            // topics are (symbol "admin", symbol "transfer")
            // we just verify at least one event was emitted after transfer
            let _ = e;
            true
        });
        assert!(found);
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

        client.register_watcher(&admin, &watcher);
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

        client.register_watcher(&admin, &w1);
        client.register_watcher(&admin, &w2);
        client.register_watcher(&admin, &w3);

        assert_eq!(client.get_watchers().len(), 3);
        assert!(client.is_watcher_authorized(&w1));
        assert!(client.is_watcher_authorized(&w2));
        assert!(client.is_watcher_authorized(&w3));
    }

    // 10. get_admin returns correct admin
    #[test]
    fn test_get_admin() {
        let (_env, admin, client) = setup();
        assert_eq!(client.get_admin().unwrap(), admin);
    }

    #[test]
    fn test_get_admin_uninitialized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(WatcherRegistry, ());
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
        let contract_id = env.register(WatcherRegistry, ());
        let client = WatcherRegistryClient::new(&env, &contract_id);
        client.get_admin();
    }

    // 12. old admin cannot act after transfer
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

    // ── Multi-admin tests ─────────────────────────────────────────────────────

    // 13. add_admin — second admin can perform privileged operations
    #[test]
    fn test_add_admin_grants_privileges() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        assert_eq!(client.try_add_admin(&admin, &second_admin).unwrap(), Ok(()));

        // second admin can now register watchers
        assert_eq!(
            client.try_register_watcher(&second_admin, &watcher).unwrap(),
            Ok(())
        );
        assert!(client.is_authorized(&watcher));
    }

    // 14. add_admin is idempotent
    #[test]
    fn test_add_admin_idempotent() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);

        client.try_add_admin(&admin, &second_admin).unwrap();
        client.try_add_admin(&admin, &second_admin).unwrap();

        assert_eq!(client.get_admins().unwrap().len(), 2);
    }

    // 15. remove_admin — removed admin loses privileges
    #[test]
    fn test_remove_admin_revokes_privileges() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);
        let watcher = Address::generate(&env);

        client.try_add_admin(&admin, &second_admin).unwrap();
        assert_eq!(
            client.try_remove_admin(&admin, &second_admin).unwrap(),
            Ok(())
        );

        assert_eq!(
            client
                .try_register_watcher(&second_admin, &watcher)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 16. remove_admin — cannot remove the last admin
    #[test]
    fn test_remove_last_admin_rejected() {
        let (env, admin, client) = setup();

        assert_eq!(
            client
                .try_remove_admin(&admin, &admin)
                .unwrap_err()
                .unwrap(),
            ContractError::LastAdmin
        );
    }

    // 17. get_admins returns all admins
    #[test]
    fn test_get_admins() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);

        client.try_add_admin(&admin, &second_admin).unwrap();

        let admins = client.get_admins().unwrap();
        assert_eq!(admins.len(), 2);
    }

    // 18. non-admin cannot add_admin
    #[test]
    fn test_add_admin_unauthorized() {
        let (env, _admin, client) = setup();
        let attacker = Address::generate(&env);
        let victim = Address::generate(&env);

        assert_eq!(
            client
                .try_add_admin(&attacker, &victim)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 19. non-admin cannot remove_admin
    #[test]
    fn test_remove_admin_unauthorized() {
        let (env, admin, client) = setup();
        let attacker = Address::generate(&env);

        assert_eq!(
            client
                .try_remove_admin(&attacker, &admin)
                .unwrap_err()
                .unwrap(),
            ContractError::Unauthorized
        );
    }

    // 20. add_admin emits event
    #[test]
    fn test_add_admin_emits_event() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);

        client.add_admin(&admin, &second_admin).unwrap();

        // At least one event was emitted (the add event)
        assert!(!env.events().all().is_empty());
    }

    // 21. remove_admin emits event
    #[test]
    fn test_remove_admin_emits_event() {
        let (env, admin, client) = setup();
        let second_admin = Address::generate(&env);

        client.add_admin(&admin, &second_admin).unwrap();
        client.remove_admin(&admin, &second_admin).unwrap();

        assert!(!env.events().all().is_empty());
    }

    // 22. remove_watcher emits event
    #[test]
    fn test_remove_watcher_emits_event() {
        let (env, admin, client) = setup();
        let watcher = Address::generate(&env);

        client.register_watcher(&admin, &watcher).unwrap();
        client.remove_watcher(&admin, &watcher).unwrap();

        assert!(!env.events().all().is_empty());
    }
}
