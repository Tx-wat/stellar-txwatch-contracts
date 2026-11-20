//! Tests for `get_alerts_modified_since` when the ID range has removed entries (#483).
//!
//! The scan starts at ID 0 and runs up to `NEXT_ID`. These tests check that results stay
//! correct when removed IDs sit at the head, in the middle, or in the tail of that range.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, vec, Address, Env, String};

use crate::{AlertRegistry, AlertRegistryClient};

fn setup() -> (Env, AlertRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AlertRegistry, ());
    (env.clone(), AlertRegistryClient::new(&env, &contract_id))
}

fn register(env: &Env, client: &AlertRegistryClient<'static>, owner: &Address, label: &str) -> u64 {
    let target = Address::generate(env);
    client.register_alert(
        owner,
        &target,
        &String::from_str(env, label),
        &String::from_str(
            env,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        &vec![env, String::from_str(env, "rule:transfer")],
        &None,
    )
}

#[test]
fn removed_tail_ids_are_not_returned() {
    let (env, client) = setup();
    let owner = Address::generate(&env);

    env.ledger().set_timestamp(100);
    let a = register(&env, &client, &owner, "a");
    env.ledger().set_timestamp(200);
    let b = register(&env, &client, &owner, "b");
    env.ledger().set_timestamp(300);
    let c = register(&env, &client, &owner, "c");

    client.remove_alert(&owner, &c);

    let results = client.get_alerts_modified_since(&0u64);
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(0).unwrap().label, String::from_str(&env, "a"));
    assert_eq!(results.get(1).unwrap().label, String::from_str(&env, "b"));
}

#[test]
fn removed_head_ids_are_skipped_and_live_tail_is_kept() {
    let (env, client) = setup();
    let owner = Address::generate(&env);

    env.ledger().set_timestamp(100);
    let a = register(&env, &client, &owner, "a");
    env.ledger().set_timestamp(200);
    let b = register(&env, &client, &owner, "b");
    env.ledger().set_timestamp(300);
    register(&env, &client, &owner, "c");

    client.remove_alert(&owner, &a);
    client.remove_alert(&owner, &b);

    let results = client.get_alerts_modified_since(&0u64);
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().label, String::from_str(&env, "c"));
}

#[test]
fn fully_removed_range_returns_empty_even_with_high_next_id() {
    let (env, client) = setup();
    let owner = Address::generate(&env);

    let mut ids = [0u64; 5];
    for slot in ids.iter_mut() {
        *slot = register(&env, &client, &owner, "x");
    }
    for id in ids {
        client.remove_alert(&owner, &id);
    }

    assert_eq!(
        client.get_alert_count(),
        5,
        "NEXT_ID keeps the removed range"
    );
    assert_eq!(client.get_alerts_modified_since(&0u64).len(), 0);
}

#[test]
fn since_filter_still_applies_across_removed_gaps() {
    let (env, client) = setup();
    let owner = Address::generate(&env);

    env.ledger().set_timestamp(100);
    register(&env, &client, &owner, "a");
    env.ledger().set_timestamp(200);
    let b = register(&env, &client, &owner, "b");
    env.ledger().set_timestamp(300);
    let c = register(&env, &client, &owner, "c");
    env.ledger().set_timestamp(400);
    let d = register(&env, &client, &owner, "d");

    client.remove_alert(&owner, &b);

    let results = client.get_alerts_modified_since(&250u64);
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(0).unwrap().label, String::from_str(&env, "c"));
    assert_eq!(results.get(1).unwrap().label, String::from_str(&env, "d"));
}
