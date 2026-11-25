//! Tests for `batch_remove_watchers` (#421).

#![cfg(test)]

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    Address, Env, IntoVal, Val, Vec,
};

use crate::{ContractError, WatcherRegistry, WatcherRegistryClient, MAX_BATCH_SIZE};

fn setup() -> (Env, WatcherRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WatcherRegistry, ());
    let client = WatcherRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

/// Registers `n` fresh watchers and returns them in registration order.
fn register_many(
    env: &Env,
    client: &WatcherRegistryClient<'_>,
    admin: &Address,
    n: u32,
) -> Vec<Address> {
    let mut out = Vec::new(env);
    for _ in 0..n {
        let w = Address::generate(env);
        client.register_watcher(admin, &w);
        out.push_back(w);
    }
    out
}

/// Number of `("watcher", "remove")` events emitted by the most recent call.
fn remove_event_count(env: &Env) -> u32 {
    let topic_watcher: Val = symbol_short!("watcher").into_val(env);
    let topic_remove: Val = symbol_short!("remove").into_val(env);
    env.events()
        .all()
        .iter()
        .filter(|(_, topics, _)| {
            topics.len() == 2
                && topics.get(0).unwrap().shallow_eq(&topic_watcher)
                && topics.get(1).unwrap().shallow_eq(&topic_remove)
        })
        .count() as u32
}

#[test]
fn removes_only_the_listed_watchers_and_returns_the_count() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 4);

    let mut targets = Vec::new(&env);
    targets.push_back(watchers.get(0).unwrap());
    targets.push_back(watchers.get(2).unwrap());

    let removed = client.batch_remove_watchers(&admin, &targets);

    assert_eq!(removed, 2);
    assert!(!client.is_authorized(&watchers.get(0).unwrap()));
    assert!(!client.is_authorized(&watchers.get(2).unwrap()));
    assert!(client.is_authorized(&watchers.get(1).unwrap()));
    assert!(client.is_authorized(&watchers.get(3).unwrap()));
}

#[test]
fn unregistered_addresses_are_skipped_without_error() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 2);
    let stranger = Address::generate(&env);

    let mut targets = Vec::new(&env);
    targets.push_back(stranger);
    targets.push_back(watchers.get(0).unwrap());

    let removed = client.batch_remove_watchers(&admin, &targets);

    assert_eq!(removed, 1);
    assert_eq!(client.get_watcher_count(), 1);
}

#[test]
fn duplicate_addresses_in_the_batch_are_removed_once() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 2);
    let target = watchers.get(0).unwrap();

    let mut targets = Vec::new(&env);
    targets.push_back(target.clone());
    targets.push_back(target.clone());

    assert_eq!(client.batch_remove_watchers(&admin, &targets), 1);
    assert_eq!(client.get_watcher_count(), 1);
}

#[test]
fn decrements_the_watcher_count_by_the_number_removed() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 5);
    assert_eq!(client.get_watcher_count(), 5);

    let mut targets = Vec::new(&env);
    for i in 0..3 {
        targets.push_back(watchers.get(i).unwrap());
    }

    client.batch_remove_watchers(&admin, &targets);

    assert_eq!(client.get_watcher_count(), 2);
}

#[test]
fn emits_one_remove_event_per_watcher_actually_removed() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 3);
    let stranger = Address::generate(&env);

    let mut targets = Vec::new(&env);
    targets.push_back(watchers.get(0).unwrap());
    targets.push_back(stranger);
    targets.push_back(watchers.get(1).unwrap());

    client.batch_remove_watchers(&admin, &targets);

    assert_eq!(remove_event_count(&env), 2);
}

#[test]
fn batch_of_exactly_max_batch_size_is_accepted() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, MAX_BATCH_SIZE);

    let removed = client.batch_remove_watchers(&admin, &watchers);

    assert_eq!(removed, MAX_BATCH_SIZE);
    assert_eq!(client.get_watcher_count(), 0);
}

#[test]
fn batch_above_max_batch_size_is_rejected_and_changes_nothing() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, MAX_BATCH_SIZE + 1);

    assert_eq!(
        client.try_batch_remove_watchers(&admin, &watchers),
        Err(Ok(ContractError::BatchTooLarge))
    );
    assert_eq!(client.get_watcher_count(), MAX_BATCH_SIZE + 1);
    assert!(client.is_authorized(&watchers.get(0).unwrap()));
}

#[test]
fn empty_batch_returns_zero_and_emits_nothing() {
    let (env, client, admin) = setup();
    register_many(&env, &client, &admin, 2);

    let empty: Vec<Address> = Vec::new(&env);

    assert_eq!(client.batch_remove_watchers(&admin, &empty), 0);
    assert_eq!(client.get_watcher_count(), 2);
    assert_eq!(remove_event_count(&env), 0);
}

#[test]
fn non_admin_cannot_batch_remove() {
    let (env, client, admin) = setup();
    let watchers = register_many(&env, &client, &admin, 1);
    let stranger = Address::generate(&env);

    assert_eq!(
        client.try_batch_remove_watchers(&stranger, &watchers),
        Err(Ok(ContractError::Unauthorized))
    );
    assert!(client.is_authorized(&watchers.get(0).unwrap()));
}
