//! Tests for `get_active_alert_count` across registrations, removals, and owners (#484).
//!
//! The count is computed by checking each owned ID. These tests pin the observable result so a
//! cached counter can be introduced without changing what callers see.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn register(env: &Env, client: &AlertRegistryClient<'static>, owner: &Address) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, "alert"),
        &String::from_str(
            env,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn new_owner_has_zero_active_alerts() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    assert_eq!(client.get_active_alert_count(&owner), 0);
}

#[test]
fn count_tracks_each_registration() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    for expected in 1..=50u32 {
        register(&env, &client, &owner);
        assert_eq!(client.get_active_alert_count(&owner), expected);
    }
}

#[test]
fn count_decrements_on_each_removal() {
    let (env, client) = setup();
    let owner = Address::generate(&env);
    let ids: [u64; 4] = [
        register(&env, &client, &owner),
        register(&env, &client, &owner),
        register(&env, &client, &owner),
        register(&env, &client, &owner),
    ];

    client.remove_alert(&owner, &ids[1]);
    assert_eq!(client.get_active_alert_count(&owner), 3);
    client.remove_alert(&owner, &ids[3]);
    assert_eq!(client.get_active_alert_count(&owner), 2);
    client.remove_alert(&owner, &ids[0]);
    assert_eq!(client.get_active_alert_count(&owner), 1);
    client.remove_alert(&owner, &ids[2]);
    assert_eq!(client.get_active_alert_count(&owner), 0);
}

#[test]
fn counts_are_isolated_per_owner() {
    let (env, client) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let a1 = register(&env, &client, &alice);
    register(&env, &client, &alice);
    register(&env, &client, &bob);

    client.remove_alert(&alice, &a1);

    assert_eq!(client.get_active_alert_count(&alice), 1);
    assert_eq!(client.get_active_alert_count(&bob), 1);
}
