//! Unit tests for `WatcherRegistry` admin and watcher operations (#260).

#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Events as _, Ledger as _}, Address, Env, IntoVal, Symbol};

use crate::{ContractError, WatcherRegistry, WatcherRegistryClient};

fn setup() -> (Env, WatcherRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WatcherRegistry, ());
    let client = WatcherRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn initialize_sets_the_admin() {
    let (_env, client, admin) = setup();
    assert_eq!(client.get_admins().len(), 1);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn double_initialize_is_rejected() {
    let (env, client, _admin) = setup();
    let other = Address::generate(&env);
    assert_eq!(client.try_initialize(&other), Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
fn non_admin_cannot_add_an_admin() {
    let (env, client, _admin) = setup();
    let stranger = Address::generate(&env);
    let new_admin = Address::generate(&env);
    assert_eq!(client.try_add_admin(&stranger, &new_admin), Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn duplicate_add_admin_is_idempotent() {
    let (env, client, admin) = setup();
    let second = Address::generate(&env);
    client.add_admin(&admin, &second);
    client.add_admin(&admin, &second);
    assert_eq!(client.get_admins().len(), 2);
}

#[test]
fn re_adding_the_same_admin_is_a_noop() {
    let (_env, client, admin) = setup();
    client.add_admin(&admin, &admin);
    assert_eq!(client.get_admins().len(), 1);
}

#[test]
fn removing_the_last_admin_is_refused() {
    let (_env, client, admin) = setup();
    assert_eq!(client.try_remove_admin(&admin, &admin), Err(Ok(ContractError::LastAdmin)));
}

#[test]
fn remove_admin_for_non_admin_returns_not_admin_and_changes_nothing() {
    let (env, client, admin) = setup();
    let second = Address::generate(&env);
    client.add_admin(&admin, &second);
    let stranger = Address::generate(&env);

    assert_eq!(
        client.try_remove_admin(&admin, &stranger),
        Err(Ok(ContractError::NotAdmin))
    );
    let admins = client.get_admins();
    assert_eq!(admins.len(), 2);
    assert!(admins.contains(&admin));
    assert!(admins.contains(&second));
}

#[test]
fn transfer_admin_replaces_the_admin_set() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&admin, &new_admin);
    let admins = client.get_admins();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins.get(0).unwrap(), new_admin);
}

#[test]
fn register_watcher_is_idempotent_and_admin_only() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    client.register_watcher(&admin, &watcher);
    client.register_watcher(&admin, &watcher);
    assert_eq!(client.get_watcher_count(), 1);

    let stranger = Address::generate(&env);
    assert_eq!(client.try_register_watcher(&stranger, &watcher), Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn remove_unknown_watcher_returns_not_found() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    assert_eq!(client.try_remove_watcher(&admin, &watcher), Err(Ok(ContractError::WatcherNotFound)));
}

#[test]
fn clear_all_watchers_empties_the_list() {
    let (env, client, admin) = setup();
    client.register_watcher(&admin, &Address::generate(&env));
    client.register_watcher(&admin, &Address::generate(&env));
    client.clear_all_watchers(&admin);
    assert_eq!(client.get_watcher_count(), 0);
}

#[test]
fn clear_all_watchers_emits_one_remove_event_per_watcher() {
    let (env, client, admin) = setup();
    client.register_watcher(&admin, &Address::generate(&env), &None);
    client.register_watcher(&admin, &Address::generate(&env), &None);
    client.register_watcher(&admin, &Address::generate(&env), &None);
    assert_eq!(client.get_watcher_count(), 3);

    client.clear_all_watchers(&admin);

    let remove_topic = (Symbol::new(&env, "watcher"), Symbol::new(&env, "remove"));
    let remove_events = env
        .events()
        .all()
        .iter()
        .filter(|event| event.topics == remove_topic.into_val(&env))
        .count();
    assert_eq!(remove_events, 3);
    assert_eq!(client.get_watcher_count(), 0);
}

#[test]
fn is_authorized_is_true_only_for_registered_watchers() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    let other = Address::generate(&env);
    client.register_watcher(&admin, &watcher);
    assert!(client.is_authorized(&watcher));
    assert!(!client.is_authorized(&other));
}

// ── #424: is_watcher_authorized_at(watcher, at) ─────────────────────────────

#[test]
fn is_watcher_authorized_at_true_when_no_expiry_is_set() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    client.register_watcher(&admin, &watcher, &None, &None);
    assert!(client.is_watcher_authorized_at(&watcher, &0));
    assert!(client.is_watcher_authorized_at(&watcher, &u64::MAX));
}

#[test]
fn is_watcher_authorized_at_respects_the_expiry_boundary() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    client.register_watcher(&admin, &watcher, &None, &Some(200));
    // Inclusive boundary: at == expiry is still authorized.
    assert!(client.is_watcher_authorized_at(&watcher, &200));
    assert!(client.is_watcher_authorized_at(&watcher, &199));
    assert!(!client.is_watcher_authorized_at(&watcher, &201));
}

#[test]
fn is_watcher_authorized_at_false_for_unregistered_watcher() {
    let (env, client, _admin) = setup();
    let stranger = Address::generate(&env);
    assert!(!client.is_watcher_authorized_at(&stranger, &0));
}

#[test]
fn is_watcher_authorized_at_is_independent_of_current_ledger_time() {
    use soroban_sdk::testutils::Ledger as _;
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    env.ledger().set_timestamp(100);
    client.register_watcher(&admin, &watcher, &None, &Some(200));
    // Current time is past expiry, but the historical query still answers for T=150.
    env.ledger().set_timestamp(10_000);
    assert!(!client.is_watcher_authorized(&watcher));
    assert!(client.is_watcher_authorized_at(&watcher, &150));
    assert!(!client.is_watcher_authorized_at(&watcher, &500));
}

#[test]
fn is_watcher_authorized_at_false_after_removal() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    client.register_watcher(&admin, &watcher, &None, &None);
    client.remove_watcher(&admin, &watcher);
    assert!(!client.is_watcher_authorized_at(&watcher, &0));
}

// ── #426: get_watcher_registered_at(watcher) ────────────────────────────────

#[test]
fn get_watcher_registered_at_returns_the_registration_timestamp() {
    use soroban_sdk::testutils::Ledger as _;
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    env.ledger().set_timestamp(1_234);
    client.register_watcher(&admin, &watcher, &None, &None);
    assert_eq!(client.get_watcher_registered_at(&watcher), Some(1_234));
    // Matches the value stored in WatcherInfo.
    assert_eq!(
        client.get_watcher_info(&watcher).unwrap().registered_at,
        1_234
    );
}

#[test]
fn get_watcher_registered_at_none_for_unknown_watcher() {
    let (env, client, _admin) = setup();
    let stranger = Address::generate(&env);
    assert_eq!(client.get_watcher_registered_at(&stranger), None);
}

#[test]
fn get_watcher_registered_at_none_after_removal() {
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    client.register_watcher(&admin, &watcher, &None, &None);
    client.remove_watcher(&admin, &watcher);
    assert_eq!(client.get_watcher_registered_at(&watcher), None);
}

#[test]
fn get_watcher_registered_at_unchanged_by_later_renewal() {
    use soroban_sdk::testutils::Ledger as _;
    let (env, client, admin) = setup();
    let watcher = Address::generate(&env);
    env.ledger().set_timestamp(50);
    client.register_watcher(&admin, &watcher, &None, &None);
    env.ledger().set_timestamp(900);
    client.renew_watcher(&admin, &watcher, &Some(2_000));
    assert_eq!(client.get_watcher_registered_at(&watcher), Some(50));
}
